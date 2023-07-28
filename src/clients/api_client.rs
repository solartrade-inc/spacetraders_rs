use crate::models::*;
use hyper::Method;
use hyper::Request;
use hyper::Uri;
use log::*;
use serde_json::json;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ApiClient {
    inner: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
    base_url: String,
    auth_token: Option<String>,
}

pub struct ApiClientResponse {
    pub status: hyper::StatusCode,
    pub headers: hyper::HeaderMap,
    pub body: String,
}

impl ApiClient {
    pub fn new() -> Self {
        let https = hyper_tls::HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        let base_url = std::env::var("SPACETRADERS_API_URL").unwrap_or_else(|_| {
            panic!("SPACETRADERS_API_URL must be set");
        });
        Self {
            inner: client,
            base_url: base_url,
            auth_token: None,
        }
    }

    async fn request(&self, req: hyper::Request<hyper::Body>) -> ApiClientResponse {
        let res = self.inner.request(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();

        // trace?
        ApiClientResponse {
            status,
            headers,
            body,
        }
    }

    async fn get(&self, path: &str) -> ApiClientResponse {
        self.req(Method::GET, path, "").await
    }
    async fn patch<T: ToString>(&self, path: &str, payload: T) -> ApiClientResponse {
        self.req(Method::PATCH, path, payload).await
    }
    async fn post<T: ToString>(&self, path: &str, payload: T) -> ApiClientResponse {
        self.req(Method::POST, path, payload).await
    }

    async fn req<T: ToString>(&self, method: Method, path: &str, payload: T) -> ApiClientResponse {
        debug!("{} {}", method, path);
        let uri: Uri = format!("{}{}", self.base_url, path).parse().unwrap();
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(auth_token) = &self.auth_token {
            req = req.header("Authorization", format!("Bearer {}", auth_token));
        }
        let body: String = payload.to_string();
        let req = match body.is_empty() {
            false => req
                .header("Content-Type", "application/json")
                .body(hyper::Body::from(body)),
            true => req.header("Content-Length", "0").body(hyper::Body::empty()),
        }
        .unwrap();
        self.request(req).await
    }

    pub fn set_auth_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }

    pub async fn register(
        &self,
        callsign: &str,
        faction: &str,
        email: Option<&str>,
    ) -> ApiClientResponse {
        let mut payload = json!({
            "faction": faction,
            "symbol": callsign,
        });
        if let Some(email) = email {
            payload["email"] = json!(email);
        }
        self.post("/v2/register", payload).await
    }

    pub async fn survey(&self, ship_symbol: &str) -> (Vec<Survey>, ShipCooldown) {
        let resp = self
            .post(&format!("/v2/my/ships/{}/survey", ship_symbol), "")
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to survey: {} {}",
            resp.status,
            resp.body
        );
        // 409 Conflict {"error":{"message":"Ship action is still on cooldown for 18 second(s).","code":4000,"data":{"cooldown":{"shipSymbol":"SOLARTRADE_INC-3","totalSeconds":70,"remainingSeconds":18,"expiration":"2023-07-23T13:22:33.774Z"}}}}

        let mut body: Value = serde_json::from_str(&resp.body).unwrap();
        let surveys: Vec<Survey> = serde_json::from_value(body["data"]["surveys"].take())
            .unwrap_or_else(|e| {
                error!(
                    "Deserialization error: '{}' while parsing surveys\n{}",
                    e, resp.body
                );
                panic!();
            });
        let cooldown: ShipCooldown = serde_json::from_value(body["data"]["cooldown"].take())
            .unwrap_or_else(|e| {
                error!(
                    "Deserialization error: '{}' while parsing cooldown\n{}",
                    e, resp.body
                );
                panic!();
            });
        (surveys, cooldown)
    }

    pub async fn extract(
        &self,
        ship_symbol: &str,
        survey: Option<&Survey>,
    ) -> Result<(ShipExtraction, ShipCooldown, ShipCargo), ApiError> {
        let req_body = match survey {
            Some(survey) => json!({
                "survey": survey,
            }),
            None => json!({}),
        };
        let resp = self
            .post(&format!("/v2/my/ships/{}/extract", ship_symbol), req_body)
            .await;

        match resp.status.as_u16() {
            200..=299 => {
                let mut body: Value = serde_json::from_str(&resp.body).unwrap();
                let cooldown: ShipCooldown =
                    serde_json::from_value(body["data"]["cooldown"].take()).unwrap_or_else(|e| {
                        error!(
                            "Deserialization error: '{}' while parsing cooldown\n{}",
                            e, resp.body
                        );
                        panic!();
                    });
                let cargo: ShipCargo = serde_json::from_value(body["data"]["cargo"].take())
                    .unwrap_or_else(|e| {
                        error!(
                            "Deserialization error: '{}' while parsing cargo\n{}",
                            e, resp.body
                        );
                        panic!();
                    });
                let extraction: ShipExtraction =
                    serde_json::from_value(body["data"]["extraction"].take()).unwrap_or_else(|e| {
                        error!(
                            "Deserialization error: '{}' while parsing extraction\n{}",
                            e, resp.body
                        );
                        panic!();
                    });
                Ok((extraction, cooldown, cargo))
            }
            400 | 409 => {
                // 409 Conflict {"error":{"message":"Ship extract failed. Survey X1-JK96-45265A-AF05A7 has been exhausted.","code":4224}}
                // 400 Bad Request {"error":{"message":"Ship survey failed. Target signature is no longer in range or valid.","code":4221}}
                let mut body: Value = serde_json::from_str(&resp.body).unwrap();
                let error: ApiError =
                    serde_json::from_value(body["error"].take()).unwrap_or_else(|e| {
                        error!(
                            "Deserialization error: '{}' while parsing error\n{}",
                            e, resp.body
                        );
                        panic!();
                    });
                Err(error)
            }
            _ => {
                panic!("Failed to extract: {} {}", resp.status, resp.body);
            }
        }
    }

    pub async fn fetch_agent(&self) {
        let resp = self.get("/v2/my/agent").await;
        assert!(
            resp.status.is_success(),
            "Failed to fetch agent: {} {}",
            resp.status,
            resp.body
        );
    }

    pub async fn fetch_contracts(&self, page: u32, limit: u32) {
        let resp = self
            .get(&format!("/v2/my/contracts?page={}&limit={}", page, limit))
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to fetch contracts: {} {}",
            resp.status,
            resp.body
        );
    }

    pub async fn fetch_ships(&self, page: u32, limit: u32) -> List<Ship> {
        let resp = self
            .get(&format!("/v2/my/ships?page={}&limit={}", page, limit))
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to fetch ships: {} {}",
            resp.status,
            resp.body
        );
        let ships: List<Ship> = serde_json::from_str(&resp.body).unwrap_or_else(|e| {
            error!(
                "Deserialization error: '{}' while parsing ships\n{}",
                e, resp.body
            );
            panic!();
        });
        ships
    }

    pub async fn fetch_system_waypoints(&self, system_symbol: &str) -> Vec<Waypoint> {
        let page = 1;
        let limit = 20;
        let resp = self
            .get(&format!(
                "/v2/systems/{}/waypoints?page={}&limit={}",
                system_symbol, page, limit
            ))
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to fetch system waypoints: {} {}",
            resp.status,
            resp.body
        );
        let waypoints: List<Waypoint> = serde_json::from_str(&resp.body).unwrap_or_else(|e| {
            error!(
                "Deserialization error: '{}' while parsing system waypoints\n{}",
                e, resp.body
            );
            panic!();
        });
        assert_eq!(waypoints.meta.page, page);
        assert_eq!(waypoints.meta.limit, limit);
        assert!(waypoints.meta.total <= 20);
        waypoints.data
    }

    pub async fn flight_mode(&self, ship_symbol: &str, flight_mode: &str) -> ShipNav {
        let resp = self
            .patch(
                &format!("/v2/my/ships/{}/nav", ship_symbol),
                json! ({
                    "flightMode": flight_mode,
                }),
            )
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to set flight mode: {} {}",
            resp.status,
            resp.body
        );
        let nav: Data<ShipNav> = serde_json::from_str(&resp.body).unwrap_or_else(|e| {
            error!(
                "Deserialization error: '{}' while parsing nav\n{}",
                e, resp.body
            );
            panic!();
        });
        nav.data
    }

    pub async fn orbit(&self, ship_symbol: &str) -> ShipNav {
        let resp = self
            .post(&format!("/v2/my/ships/{}/orbit", ship_symbol), "")
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to orbit: {} {}",
            resp.status,
            resp.body
        );
        let mut body: Value = serde_json::from_str(&resp.body).unwrap();
        let nav: ShipNav = serde_json::from_value(body["data"]["nav"].take()).unwrap_or_else(|e| {
            error!(
                "Deserialization error: '{}' while parsing nav\n{}",
                e, resp.body
            );
            panic!();
        });
        nav
    }

    pub async fn dock(&self, ship_symbol: &str) -> ShipNav {
        let resp = self
            .post(&format!("/v2/my/ships/{}/dock", ship_symbol), "")
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to dock: {} {}",
            resp.status,
            resp.body
        );
        let mut body: Value = serde_json::from_str(&resp.body).unwrap();
        let nav: ShipNav = serde_json::from_value(body["data"]["nav"].take()).unwrap_or_else(|e| {
            error!(
                "Deserialization error: '{}' while parsing nav\n{}",
                e, resp.body
            );
            panic!();
        });
        nav
    }

    pub async fn navigate(&self, ship_symbol: &str, waypoint_symbol: &str) -> (ShipNav, ShipFuel) {
        let resp = self
            .post(
                &format!("/v2/my/ships/{}/navigate", ship_symbol),
                json! ({
                    "waypointSymbol": waypoint_symbol,
                }),
            )
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to navigate: {} {}",
            resp.status,
            resp.body
        );
        let mut body: Value = serde_json::from_str(&resp.body).unwrap();
        let nav: ShipNav = serde_json::from_value(body["data"]["nav"].take()).unwrap_or_else(|e| {
            error!(
                "Deserialization error: '{}' while parsing nav\n{}",
                e, resp.body
            );
            panic!();
        });
        let fuel: ShipFuel =
            serde_json::from_value(body["data"]["fuel"].take()).unwrap_or_else(|e| {
                error!(
                    "Deserialization error: '{}' while parsing fuel\n{}",
                    e, resp.body
                );
                panic!();
            });
        (nav, fuel)
    }

    pub async fn refuel(&self, ship_symbol: &str, units: u32) -> (Agent, ShipFuel) {
        let resp = self
            .post(
                &format!("/v2/my/ships/{}/refuel", ship_symbol),
                json!({
                    "units": units,
                }),
            )
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to refuel: {} {}",
            resp.status,
            resp.body
        );
        let mut body: Value = serde_json::from_str(&resp.body).unwrap();
        let agent: Agent =
            serde_json::from_value(body["data"]["agent"].take()).unwrap_or_else(|e| {
                error!(
                    "Deserialization error: '{}' while parsing agent\n{}",
                    e, resp.body
                );
                panic!();
            });
        let fuel: ShipFuel =
            serde_json::from_value(body["data"]["fuel"].take()).unwrap_or_else(|e| {
                error!(
                    "Deserialization error: '{}' while parsing fuel\n{}",
                    e, resp.body
                );
                panic!();
            });
        (agent, fuel)
    }

    pub async fn sell(
        &self,
        ship_symbol: &str,
        symbol: &str,
        units: u32,
    ) -> (Agent, ShipCargo, MarketTransaction) {
        let resp = self
            .post(
                &format!("/v2/my/ships/{}/sell", ship_symbol),
                json!({
                    "symbol": symbol,
                    "units": units,
                }),
            )
            .await;
        assert!(
            resp.status.is_success(),
            "Failed to sell: {} {}",
            resp.status,
            resp.body
        );
        let mut body: Value = serde_json::from_str(&resp.body).unwrap();
        let agent: Agent =
            serde_json::from_value(body["data"]["agent"].take()).unwrap_or_else(|e| {
                error!(
                    "Deserialization error: '{}' while parsing agent\n{}",
                    e, resp.body
                );
                panic!();
            });
        let cargo: ShipCargo =
            serde_json::from_value(body["data"]["cargo"].take()).unwrap_or_else(|e| {
                error!(
                    "Deserialization error: '{}' while parsing cargo\n{}",
                    e, resp.body
                );
                panic!();
            });
        let transaction: MarketTransaction =
            serde_json::from_value(body["data"]["transaction"].take()).unwrap_or_else(|e| {
                error!(
                    "Deserialization error: '{}' while parsing transaction\n{}",
                    e, resp.body
                );
                panic!();
            });
        (agent, cargo, transaction)
    }

    pub async fn fetch_market(&self, system: &str, waypoint: &str) -> Market {
        let uri = format!("/v2/systems/{}/waypoints/{}/market", system, waypoint);
        let resp = self.get(&uri).await;
        assert!(
            resp.status.is_success(),
            "Failed to register agent: {} {}",
            resp.status,
            resp.body
        );
        let mut body: Value = serde_json::from_str(&resp.body).unwrap();
        let market: Market = serde_json::from_value(body["data"].take()).unwrap_or_else(|e| {
            error!(
                "Deserialization error: '{}' while parsing market\n{}",
                e, resp.body
            );
            panic!();
        });
        market
    }
}
