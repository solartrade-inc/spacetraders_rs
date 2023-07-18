use crate::models::Market;

use hyper::Uri;
use log::*;

use serde_json::json;
use serde_json::Value;

pub struct ApiClient {
    pub inner: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
    pub base_url: &'static str,
    pub auth_token: Option<String>,
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
        Self {
            inner: client,
            base_url: "https://api.spacetraders.io/v2",
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
        let uri: Uri = format!("{}{}", self.base_url, path).parse().unwrap();
        let mut req = hyper::Request::get(uri);
        if let Some(auth_token) = &self.auth_token {
            req = req.header("Authorization", format!("Bearer {}", auth_token));
        }
        let req = req.body(hyper::Body::empty()).unwrap();
        self.request(req).await
    }

    async fn post<T: ToString>(&self, path: &str, payload: T) -> ApiClientResponse {
        let uri: Uri = format!("{}{}", self.base_url, path).parse().unwrap();
        let req = hyper::Request::post(uri)
            .header("Content-Type", "application/json")
            .body(hyper::Body::from(payload.to_string()))
            .unwrap();
        self.request(req).await
    }

    pub async fn set_auth_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }

    pub async fn register(&self, callsign: &str, faction: &str) -> ApiClientResponse {
        self.post(
            "/register",
            json!({
                "faction": faction,
                "symbol": callsign,
            }),
        )
        .await
    }

    pub async fn fetch_market(&self, system: &str, waypoint: &str) -> Market {
        let uri = format!("/systems/{}/waypoints/{}/market", system, waypoint);
        let resp = self.get(&uri).await;
        assert!(
            resp.status.is_success(),
            "Failed to register agent: {} {}",
            resp.status,
            resp.body
        );
        let mut body: Value = serde_json::from_str(&resp.body).unwrap();
        let market: Market = serde_json::from_value(body["data"].take()).unwrap_or_else(|e| {
            error!("Decode error: '{}' while parsing market\n{}", e, resp.body);
            panic!();
        });
        market
    }
}
