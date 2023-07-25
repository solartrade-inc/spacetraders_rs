use std::sync::Arc;

use crate::api_client::ApiClient;
use crate::database::DatabaseClient;
use crate::models::*;
use chrono::{Utc};
use std::time::Duration;
use dashmap::DashMap;
use log::debug;
use tokio::sync::{RwLock as AsyncRwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct ControllerBuilder {
    callsign: String,
}
impl ControllerBuilder {
    pub async fn load(&self) -> Controller {
        let mut api_client = ApiClient::new();
        let db_client = DatabaseClient::new();

        // load agent
        let (bearer_token, agent) = db_client.load_agent(&self.callsign).await;

        // load surveys
        let surveys_list = db_client.load_surveys(0).await;
        let surveys: DashMap<String, Vec<Arc<WrappedSurvey>>> = DashMap::new();
        for survey in surveys_list.into_iter() {
            surveys
                .entry(survey.inner().symbol.clone())
                .or_insert(vec![])
                .push(Arc::new(survey));
        }

        // todo: load ships

        api_client.set_auth_token(bearer_token.clone());

        Controller {
            api_client,
            db_client,
            agent: Arc::new(agent),
            ships: Arc::new(DashMap::new()),
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

    // universe
    // double lock: first lock is for the map, second lock is for the ship
    pub ships: Arc<DashMap<String, Arc<AsyncRwLock<Ship>>>>,

    pub markets: Arc<DashMap<String, Arc<Market>>>,
    pub agent: Arc<Agent>,
    pub surveys: Arc<DashMap<String, Vec<Arc<WrappedSurvey>>>>,
}

impl Controller {
    pub fn new(callsign: &str) -> ControllerBuilder {
        ControllerBuilder {
            callsign: String::from(callsign),
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

    pub fn ship_controller(&self, ship_symbol: &str) -> ShipController {
        let ship_arc = self.ships.get(ship_symbol).unwrap();
        ShipController {
            symbol: ship_symbol.to_string(),
            par: self.clone(),
            ship_arc: ship_arc.clone(),
        }
    }
}

pub struct ShipController {
    symbol: String,
    pub par: Controller,
    ship_arc: Arc<AsyncRwLock<Ship>>,
}

impl ShipController {
    pub async fn ship(&self) -> RwLockReadGuard<Ship> {
        let guard = tokio::time::timeout(Duration::from_secs(5), self.ship_arc.read()).await.expect("Timeout on ship lock");
        guard
    }
    pub async fn ship_mut(&self) -> RwLockWriteGuard<Ship> {
        let guard = tokio::time::timeout(Duration::from_secs(5), self.ship_arc.write()).await.expect("Timeout on mut ship lock");
        guard
    }

    pub async fn sleep_for_navigation(&self) {
        let ship = self.ship().await;
        // OutOfRangeError on negative duration
        if let Ok(duration) = (ship.nav.route.arrival - Utc::now()).to_std() {
            debug!(
                "Sleeping for navigation {}s",
                duration.as_millis() as f64 / 1000.0
            );
            drop(ship);
            tokio::time::sleep(duration).await;
        }
    }

    pub async fn sleep_for_cooldown(&self) {
        let ship = self.ship().await;
        if let Some(cooldown) = &ship.cooldown {
            // OutOfRangeError on negative duration
            if let Ok(duration) = (cooldown.expiration - Utc::now()).to_std() {
                debug!(
                    "Sleeping for cooldown {}s",
                    duration.as_millis() as f64 / 1000.0
                );
                drop(ship);
                tokio::time::sleep(duration).await;
            }
        }
    }

    pub async fn flight_mode(&self, target: &str) {
        let mut ship = self.ship_mut().await;
        if ship.nav.flight_mode == target {
            return;
        }
        debug!("Flight mode: {} -> {}", ship.nav.flight_mode, target);
        ship.nav = self.par.api_client.flight_mode(&self.symbol, target).await;
    }

    pub async fn orbit_status(&self, target: &str) {
        let mut ship = self.ship_mut().await;
        if ship.nav.status == target {
            return;
        }
        debug!("Orbit status: {} -> {}", ship.nav.status, target);
        let nav = match target {
            "IN_ORBIT" => self.par.api_client.orbit(&self.symbol).await,
            "DOCKED" => self.par.api_client.dock(&self.symbol).await,
            _ => panic!("Unknown orbit status: {}", target),
        };
        ship.nav = nav;
        assert_eq!(ship.nav.status, target);
    }

    pub async fn navigate(&self, target: &str) {
        self.orbit_status("IN_ORBIT").await;
        let mut ship = self.ship_mut().await;
        if ship.nav.waypoint_symbol == target {
            return;
        }
        let (nav, fuel) = self.par.api_client.navigate(&self.symbol, target).await;
        ship.nav = nav;
        ship.fuel = fuel;
    }

    pub async fn fetch_market(&self) -> Market {
        let ship = self.ship().await;
        // fetch
        let market = self
            .par
            .api_client
            .fetch_market(&ship.nav.system_symbol, &ship.nav.waypoint_symbol)
            .await;
        // update database
        self.par.db_client.upsert_market(&market).await;
        // update memory
        self.par
            .markets
            .insert(market.symbol.clone(), Arc::new(market.clone()));
        market
    }

    pub async fn survey(&self) {
        self.orbit_status("IN_ORBIT").await;
        self.sleep_for_cooldown().await;
    
        let mut ship = self.ship_mut().await;
        let (surveys, cooldown) = self.par.api_client.survey(&ship.symbol).await;
        ship.cooldown = Some(cooldown);

        let wrapped: Vec<WrappedSurvey> = self.par.db_client.insert_surveys(&surveys).await;
        let mut e = self
            .par
            .surveys
            .entry(ship.nav.waypoint_symbol.clone())
            .or_insert(vec![]);
        e.extend(wrapped.into_iter().map(Arc::new));
    }

    pub async fn extract_survey(&self, survey: &WrappedSurvey) {
        self.sleep_for_cooldown().await;

        let mut ship = self.ship_mut().await;
        let extract_result = self
            .par
            .api_client
            .extract(&ship.symbol, Some(survey.inner()))
            .await;
        match extract_result {
            Ok((extraction, cooldown, cargo)) => {
                debug!(
                    "Extracted {}x {}",
                    extraction._yield.units, extraction._yield.symbol
                );
                ship.cooldown = Some(cooldown);
                ship.cargo = cargo;
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
                        .entry(ship.nav.waypoint_symbol.clone())
                        .or_insert(vec![])
                        .retain(|s| s.id != survey.id);
                }
            }
        }
    }

    pub async fn refuel(&self) {
        let ship = self.ship_mut().await;
        let refuel_units = (ship.fuel.capacity - ship.fuel.current) / 100 * 100;
        if refuel_units == 0 {
            return;
        }
        debug!("Refuel: {} units", refuel_units);
        drop(ship);
        self.orbit_status("DOCKED").await;
        let (_agent, fuel) = self.par.api_client.refuel(&self.symbol, refuel_units).await;

        let mut ship = self.ship_mut().await;
        ship.fuel = fuel;
        debug!("Updated fuel: {:?}", ship.fuel.current);
    }

    pub async fn sell(&self, symbol: &str, units: u32) {
        self.orbit_status("DOCKED").await;
        let (_agent, cargo, t) = self.par.api_client.sell(&self.symbol, symbol, units).await;
        debug!("Sold {}x {}: +${}", t.units, t.trade_symbol, t.total_price);

        let mut ship = self.ship_mut().await;
        ship.cargo = cargo;
        debug!("Updated cargo: {:?}", ship.cargo);
    }
}
