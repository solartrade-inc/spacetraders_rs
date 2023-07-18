use core::panic;
use std::collections::HashMap;

use crate::api_client::ApiClient;
use crate::database::DatabaseClient;
use crate::db_models;
use crate::models::*;

use chrono::Utc;
use hyper::body::to_bytes;
use hyper::{Body, Uri};
use log::{debug, error, info};
use serde_json::json;
use serde_json::Value;

pub struct ControllerBuilder {
    callsign: String,
}
impl ControllerBuilder {
    pub async fn load(&self) -> Controller {
        let mut api_client = ApiClient::new();
        let db_client = DatabaseClient::new();
        let agent = db_client.load_agent(&self.callsign).await;
        // let ships = vec![]; // client.load_ships(&agent.symbol).await;

        api_client.auth_token = Some(agent.bearer_token.clone());

        Controller {
            api_client,
            db_client,
            agent,
            ships: HashMap::new(),
            markets: HashMap::new(),
        }
    }
}

pub struct Controller {
    pub api_client: ApiClient,
    pub db_client: DatabaseClient,
    pub ships: HashMap<String, Ship>,
    pub markets: HashMap<String, Market>,
    pub agent: db_models::Agent,
}

impl Controller {
    pub fn new(callsign: &str) -> ControllerBuilder {
        ControllerBuilder {
            callsign: String::from(callsign),
        }
    }

    pub async fn fetch_agent(&mut self) {
        let uri: Uri = "https://api.spacetraders.io/v2/my/agent"
            .to_string()
            .parse()
            .unwrap();
        let req = hyper::Request::get(uri)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.agent.bearer_token),
            )
            .body(Body::empty())
            .unwrap();
        let res = self.api_client.inner.request(req).await.unwrap();
        let status = res.status();
        assert_eq!(status, 200);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = std::str::from_utf8(&body_bytes).unwrap();
        info!("Agent: {:?}", body);
    }

    pub async fn fetch_contracts(&mut self, page: u32, limit: u32) {
        let uri: Uri = format!(
            "https://api.spacetraders.io/v2/my/contracts?page={}&limit={}",
            page, limit
        )
        .parse()
        .unwrap();
        let req = hyper::Request::get(uri)
            .header(
                "Authorization",
                format!("Bearer {}", self.agent.bearer_token),
            )
            .body(Body::empty())
            .unwrap();
        let res = self.api_client.inner.request(req).await.unwrap();
        let status = res.status();
        assert_eq!(status, 200);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = std::str::from_utf8(&body_bytes).unwrap();
        info!("Contracts: {:?}", body);
    }

    pub async fn fetch_ships(&mut self, page: u32, limit: u32) {
        let uri: Uri = format!(
            "https://api.spacetraders.io/v2/my/ships?page={}&limit={}",
            page, limit
        )
        .parse()
        .unwrap();
        let req = hyper::Request::get(uri)
            .header(
                "Authorization",
                format!("Bearer {}", self.agent.bearer_token),
            )
            .body(Body::empty())
            .unwrap();
        let res = self.api_client.inner.request(req).await.unwrap();
        let status = res.status();
        assert_eq!(status, 200);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = std::str::from_utf8(&body_bytes).unwrap();

        let ships: List<Ship> = serde_json::from_str(body).unwrap();

        info!("Ships: {:?}", ships);
        for ship in ships.data.into_iter() {
            self.ships.insert(ship.symbol.clone(), ship);
        }
    }

    pub async fn fetch_system_waypoints(&mut self, system_symbol: &str) -> Vec<Waypoint> {
        let page = 1;
        let limit = 20;
        let uri: Uri = format!(
            "https://api.spacetraders.io/v2/systems/{}/waypoints?page={}&limit={}",
            system_symbol, page, limit
        )
        .parse()
        .unwrap();
        let req = hyper::Request::get(uri)
            .header(
                "Authorization",
                format!("Bearer {}", self.agent.bearer_token),
            )
            .body(Body::empty())
            .unwrap();
        let res = self.api_client.inner.request(req).await.unwrap();
        let status = res.status();
        assert_eq!(status, 200);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = std::str::from_utf8(&body_bytes).unwrap();

        let waypoints: List<Waypoint> = serde_json::from_str(body).unwrap();
        assert_eq!(waypoints.meta.page, page);
        assert_eq!(waypoints.meta.limit, limit);
        assert!(waypoints.meta.total <= 20);

        // info!("Waypoints: {:?}", waypoints);
        waypoints.data
    }

    pub fn ship_controller(&mut self, idx: usize) -> ShipController {
        // convert idx+1 to hex
        let ship_symbol = format!("{}-{:x}", self.agent.symbol, idx);
        let _ship = self.ships.get(&ship_symbol).unwrap();
        ShipController {
            symbol: ship_symbol,
            par: self,
        }
    }
}

pub struct ShipController<'a> {
    symbol: String,
    pub par: &'a mut Controller,
}

impl<'a> ShipController<'a> {
    pub fn ship(&self) -> &Ship {
        self.par.ships.get(&self.symbol).unwrap()
    }

    pub async fn flight_mode(&mut self, target: &str) {
        let mut ship = self.par.ships.get_mut(&self.symbol).unwrap();
        if ship.nav.flight_mode == target {
            return;
        }
        debug!("Flight mode: {} -> {}", ship.nav.flight_mode, target);
        let uri: Uri = format!(
            "https://api.spacetraders.io/v2/my/ships/{}/nav",
            self.symbol
        )
        .parse()
        .unwrap();
        let payload = json! ({
            "flightMode": target,
        });
        let req = hyper::Request::patch(uri)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.par.agent.bearer_token),
            )
            .body(hyper::Body::from(payload.to_string()))
            .unwrap();
        let res = self.par.api_client.inner.request(req).await.unwrap();
        let status = res.status();
        assert_eq!(status, 200);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = std::str::from_utf8(&body_bytes).unwrap();
        let nav: Data<ShipNav> = serde_json::from_str(body).unwrap();
        ship.nav = nav.data;
    }

    pub async fn orbit_status(&mut self, target: &str) {
        let mut ship = self.par.ships.get_mut(&self.symbol).unwrap();
        if ship.nav.status == target {
            return;
        }
        debug!("Orbit status: {} -> {}", ship.nav.status, target);
        let order = match target {
            "IN_ORBIT" => "orbit",
            "DOCKED" => "dock",
            _ => panic!("Unknown orbit status: {}", target),
        };
        let uri: Uri = format!(
            "https://api.spacetraders.io/v2/my/ships/{}/{}",
            self.symbol, order
        )
        .parse()
        .unwrap();
        let req = hyper::Request::post(&uri)
            .header(
                "Authorization",
                format!("Bearer {}", self.par.agent.bearer_token),
            )
            .header("Content-Length", "0")
            .body(Body::empty())
            .unwrap();
        let res = self.par.api_client.inner.request(req).await.unwrap();
        let status = res.status();
        let mut body: Value = {
            let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
            let body = std::str::from_utf8(&body_bytes).unwrap();
            serde_json::from_str(body).unwrap()
        };
        if status != 200 {
            error!("{} POST {}", status, uri);
            error!("{}", body);
            panic!("Failed to orbit");
        }
        let nav: ShipNav = serde_json::from_value(body["data"]["nav"].take()).unwrap();
        ship.nav = nav;
        assert_eq!(ship.nav.status, target);
    }

    pub async fn navigate(&mut self, target: &str) {
        self.orbit_status("IN_ORBIT").await;
        let mut ship = self.par.ships.get_mut(&self.symbol).unwrap();
        if ship.nav.waypoint_symbol == target {
            return;
        }
        debug!("Navigate: {}", target);
        let uri: Uri = format!(
            "https://api.spacetraders.io/v2/my/ships/{}/navigate",
            self.symbol
        )
        .parse()
        .unwrap();
        let payload = json! ({
            "waypointSymbol": target,
        });
        let req = hyper::Request::post(uri)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.par.agent.bearer_token),
            )
            .body(hyper::Body::from(payload.to_string()))
            .unwrap();
        let res = self.par.api_client.inner.request(req).await.unwrap();
        let status = res.status();
        let body: Value = {
            let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
            let body = std::str::from_utf8(&body_bytes).unwrap();
            debug!("Navigate response: {}", body);
            serde_json::from_str(body).unwrap()
        };
        assert_eq!(status, 200);

        let nav: ShipNav = serde_json::from_value(body["data"]["nav"].clone()).unwrap();
        let fuel: ShipFuel = serde_json::from_value(body["data"]["fuel"].clone()).unwrap();
        ship.nav = nav;
        ship.fuel = fuel;

        let duration = (ship.nav.route.arrival - Utc::now()).to_std().unwrap();
        debug!("Sleeping for {}s", duration.as_millis() as f64 / 1000.0);
        tokio::time::sleep(duration).await;
    }

    pub async fn fetch_market(&mut self) -> Market {
        let ship = self.par.ships.get(&self.symbol).unwrap();
        // fetch
        let market = self
            .par
            .api_client
            .fetch_market(&ship.nav.system_symbol, &ship.nav.waypoint_symbol)
            .await;
        // update memory
        self.par
            .markets
            .insert(market.symbol.clone(), market.clone());
        // update database
        self.par.db_client.upsert_market(&market).await;
        market
    }

    pub async fn refuel(&mut self) {
        let ship = self.par.ships.get_mut(&self.symbol).unwrap();
        let refuel_units = (ship.fuel.capacity - ship.fuel.current) / 100;
        if refuel_units == 0 {
            return;
        }
        debug!("Refuel: {} units", refuel_units);
        self.orbit_status("DOCKED").await;
        let mut ship = self.par.ships.get_mut(&self.symbol).unwrap();
        let uri: Uri = format!(
            "https://api.spacetraders.io/v2/my/ships/{}/refuel",
            self.symbol
        )
        .parse()
        .unwrap();
        let payload = json! ({
            "units": 100 * refuel_units,
        });
        let req = hyper::Request::post(&uri)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.par.agent.bearer_token),
            )
            .body(hyper::Body::from(payload.to_string()))
            .unwrap();
        let res = self.par.api_client.inner.request(req).await.unwrap();
        let status = res.status();
        let body = {
            let body = to_bytes(res.into_body()).await.unwrap();
            String::from_utf8(body.to_vec()).unwrap()
        };
        if status != 200 {
            error!("{} POST {}", status, uri);
            error!("{}", body);
            panic!("Failed to refuel");
        }

        debug!("Refuel response: {}", body);
        let body: Value = serde_json::from_str(&body).unwrap();

        let _agent: Agent = serde_json::from_value(body["data"]["agent"].clone()).unwrap();
        let fuel: ShipFuel = serde_json::from_value(body["data"]["fuel"].clone()).unwrap();
        ship.fuel = fuel;
        debug!("Updated fuel: {:?}", ship.fuel.current);
        // @@ self.par.agent = agent;
        // let transaction: Transaction = serde_json::from_value(body["data"]["agent"].clone()).unwrap();
    }
}
