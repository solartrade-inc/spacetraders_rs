use std::sync::{Arc, Mutex};

use crate::database::DatabaseClient;
use crate::models::*;
use crate::{api_client::ApiClient, shipconfig::AgentConfig};
use chrono::Utc;
use dashmap::DashMap;
use log::debug;
use std::time::Duration;
use tokio::{
    sync::{OwnedRwLockWriteGuard, RwLock as AsyncRwLock},
    time::sleep,
};

pub struct ControllerBuilder {
    config: AgentConfig,
}
impl ControllerBuilder {
    pub async fn load(self) -> Controller {
        let mut api_client = ApiClient::new();
        let db_client = DatabaseClient::new();

        // load agent
        let agent = db_client.load_agent(&self.config.callsign).await;
        match &agent {
            Some((bearer_token, _agent)) => {
                api_client.set_auth_token(bearer_token.clone());
            }
            None => {
                debug!("Agent not found. Continuing with non-auth controller.");
            }
        }

        // load surveys
        let surveys_list = db_client.load_surveys(0).await;
        let surveys: DashMap<String, Vec<Arc<WrappedSurvey>>> = DashMap::new();
        for survey in surveys_list.into_iter() {
            surveys
                .entry(survey.inner().symbol.clone())
                .or_insert(vec![])
                .push(Arc::new(survey));
        }

        let agent = agent.map(|(_token, agent)| agent);
        // todo: load ships
        Controller {
            api_client,
            db_client,
            config: self.config,
            agent: Arc::new(Mutex::new(agent)),
            ships: Arc::new(DashMap::new()),
            contracts: Arc::new(Mutex::new(Vec::new())),
            markets: Arc::new(DashMap::new()),
            surveys: Arc::new(surveys),
        }
    }
}

#[derive(Clone)]
pub struct Controller {
    // clients
    pub api_client: ApiClient,
    pub db_client: DatabaseClient,
    pub config: AgentConfig,

    // universe
    // double lock: first lock is for the map, second lock is for the ship
    pub ships: Arc<DashMap<String, Arc<AsyncRwLock<Ship>>>>,
    pub contracts: Arc<Mutex<Vec<Arc<Contract>>>>,
    pub markets: Arc<DashMap<String, Arc<Market>>>,
    pub agent: Arc<Mutex<Option<Agent>>>,
    pub surveys: Arc<DashMap<String, Vec<Arc<WrappedSurvey>>>>,
}

impl Controller {
    pub fn new(config: &AgentConfig) -> ControllerBuilder {
        ControllerBuilder {
            config: config.clone(),
        }
    }

    pub async fn fetch_ships(&mut self, page: u32, limit: u32) {
        let ships: List<Ship> = self.api_client.fetch_ships(page, limit).await;

        // info!("Ships: {:?}", ships);
        for ship in ships.data.into_iter() {
            self.ships
                .insert(ship.symbol.clone(), Arc::new(AsyncRwLock::new(ship)));
        }
    }

    pub async fn fetch_contracts(&mut self, page: u32, limit: u32) {
        let contracts: List<Contract> = self.api_client.fetch_contracts(page, limit).await;
        for contract in contracts.data.into_iter() {
            self.contracts.lock().unwrap().push(Arc::new(contract));
        }
    }

    pub async fn fetch_agent(&self) {
        let agent = self.api_client.fetch_agent().await;
        self.agent.lock().unwrap().replace(agent);
    }

    pub async fn accept_contract(&self, contract_id: &str) {
        let (agent, contract) = self.api_client.accept_contract(contract_id).await;
        self.agent.lock().unwrap().replace(agent);
        let mut contracts = self.contracts.lock().unwrap();
        let index = contracts
            .iter()
            .position(|c| c.id == contract.id)
            .expect("Contract not found");
        contracts[index] = Arc::new(contract);
    }

    pub async fn buy_ship(&self, ship_symbol: &str, waypoint_symbol: &str) {
        debug!(
            "Buying ship {} with waypoint {}",
            ship_symbol, waypoint_symbol
        );
        let (agent, ship) = self.api_client.buy_ship(ship_symbol, waypoint_symbol).await;
        self.agent.lock().unwrap().replace(agent);
        self.ships
            .insert(ship.symbol.clone(), Arc::new(AsyncRwLock::new(ship)));
        debug!("Bought ship {}", ship_symbol);
    }

    pub async fn ship_controller(&self, ship_symbol: &str) -> ShipController {
        let ship_arc = self.ships.get(ship_symbol).unwrap().clone();
        let guard = tokio::time::timeout(Duration::from_secs(5), ship_arc.write_owned())
            .await
            .expect("Timeout on mut ship lock");
        ShipController {
            symbol: ship_symbol.to_string(),
            par: self.clone(),
            ship: guard,
        }
    }

    // pub async fn register(&self, callsign: &str, faction: &str, email: Option<&str>) {
    pub async fn register(&mut self) {
        let callsign = self.config.callsign.clone();
        let faction = self.config.faction.clone();
        let email = self.config.email.clone();
        let (token, agent) = self
            .api_client
            .register(&callsign, &faction, email.as_deref())
            .await;
        self.db_client.save_agent(&callsign, &token, &agent).await;
        self.agent.lock().unwrap().replace(agent);
    }
}

pub struct ShipController {
    symbol: String,
    pub par: Controller,
    pub ship: OwnedRwLockWriteGuard<Ship>,
}

impl ShipController {
    pub fn navigation_cooldown(&mut self) -> Option<Duration> {
        // OutOfRangeError on negative duration
        if let Ok(duration) = (self.ship.nav.route.arrival - Utc::now()).to_std() {
            return Some(duration);
        }
        None
    }

    pub fn reactor_cooldown(&mut self) -> Option<Duration> {
        if let Some(cooldown) = &self.ship.cooldown {
            // OutOfRangeError on negative duration
            if let Ok(duration) = (cooldown.expiration - Utc::now()).to_std() {
                return Some(duration);
            }
        }
        None
    }

    pub async fn sleep_for_navigation(&mut self) {
        if let Some(cooldown) = self.navigation_cooldown() {
            debug!(
                "Sleeping for navigation cooldown {}s",
                cooldown.as_millis() as f64 / 1000.0
            );
            sleep(cooldown).await;
        }
    }

    pub async fn sleep_for_cooldown(&mut self) {
        if let Some(cooldown) = self.reactor_cooldown() {
            debug!(
                "Sleeping for reactor cooldown {}s",
                cooldown.as_millis() as f64 / 1000.0
            );
            sleep(cooldown).await;
        }
    }

    pub async fn flight_mode(&mut self, target: &str) {
        if self.ship.nav.flight_mode == target {
            return;
        }
        debug!("Flight mode: {} -> {}", self.ship.nav.flight_mode, target);
        self.ship.nav = self.par.api_client.flight_mode(&self.symbol, target).await;
    }

    pub async fn orbit_status(&mut self, target: &str) {
        if self.ship.nav.status == target {
            return;
        }
        debug!("Orbit status: {} -> {}", self.ship.nav.status, target);
        let nav = match target {
            "IN_ORBIT" => self.par.api_client.orbit(&self.symbol).await,
            "DOCKED" => self.par.api_client.dock(&self.symbol).await,
            _ => panic!("Unknown orbit status: {}", target),
        };
        self.ship.nav = nav;
        assert_eq!(self.ship.nav.status, target);
    }

    pub async fn navigate(&mut self, target: &str) {
        self.orbit_status("IN_ORBIT").await;
        if self.ship.nav.waypoint_symbol == target {
            return;
        }
        let (nav, fuel) = self.par.api_client.navigate(&self.symbol, target).await;
        self.ship.nav = nav;
        self.ship.fuel = fuel;
    }

    pub async fn fetch_market(&self) -> Market {
        // fetch
        let market = self
            .par
            .api_client
            .fetch_market(&self.ship.nav.system_symbol, &self.ship.nav.waypoint_symbol)
            .await;
        // update database
        self.par.db_client.upsert_market(&market).await;
        // update memory
        self.par
            .markets
            .insert(market.symbol.clone(), Arc::new(market.clone()));
        market
    }

    pub async fn survey(&mut self) {
        self.orbit_status("IN_ORBIT").await;

        let (surveys, cooldown) = self.par.api_client.survey(&self.ship.symbol).await;
        self.ship.cooldown = Some(cooldown);

        let wrapped: Vec<WrappedSurvey> = self.par.db_client.insert_surveys(&surveys).await;
        let mut e = self
            .par
            .surveys
            .entry(self.ship.nav.waypoint_symbol.clone())
            .or_insert(vec![]);
        e.extend(wrapped.into_iter().map(Arc::new));
    }

    pub async fn extract_survey(&mut self, survey: &WrappedSurvey) {
        let extract_result = self
            .par
            .api_client
            .extract(&self.ship.symbol, Some(survey.inner()))
            .await;
        match extract_result {
            Ok((extraction, cooldown, cargo)) => {
                debug!(
                    "Extracted {}x {}",
                    extraction._yield.units, extraction._yield.symbol
                );
                self.ship.cooldown = Some(cooldown);
                self.ship.cargo = cargo;
            }
            Err(e) => {
                debug!("Extraction failed: {:?}", e);
                if e.code == 4224 || e.code == 4221 {
                    // depleted survey or expired survey
                    debug!("Removing from database");
                    self.par.db_client.update_survey_state(survey, 2).await;
                    // remove from self.par.surveys as well
                    self.par
                        .surveys
                        .entry(self.ship.nav.waypoint_symbol.clone())
                        .or_insert(vec![])
                        .retain(|s| s.id != survey.id);
                }
                if e.code == 4000 {
                    // ship action on cooldown
                    debug!("Ship action on cooldown.. sleeping for 15s");
                    sleep(Duration::from_secs(15)).await;
                }
            }
        }
    }

    pub async fn refuel(&mut self) {
        let refuel_units = (self.ship.fuel.capacity - self.ship.fuel.current) / 100 * 100;
        if refuel_units == 0 {
            return;
        }
        debug!("Refuel: {} units", refuel_units);
        self.orbit_status("DOCKED").await;
        let (_agent, fuel) = self.par.api_client.refuel(&self.symbol, refuel_units).await;

        self.ship.fuel = fuel;
        debug!("Updated fuel: {:?}", self.ship.fuel.current);
    }

    pub async fn sell(&mut self, symbol: &str, units: u32) {
        self.orbit_status("DOCKED").await;
        let (_agent, cargo, t) = self.par.api_client.sell(&self.symbol, symbol, units).await;
        debug!("Sold {}x {}: +${}", t.units, t.trade_symbol, t.total_price);

        self.ship.cargo = cargo;
        debug!("Updated cargo: {:?}", self.ship.cargo);
    }
}
