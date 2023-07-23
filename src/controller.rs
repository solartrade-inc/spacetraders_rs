use core::panic;
use std::collections::HashMap;

use crate::api_client::ApiClient;
use crate::database::DatabaseClient;
use crate::db_models;
use crate::models::*;

use chrono::Utc;

use log::debug;

pub struct ControllerBuilder {
    callsign: String,
}
impl ControllerBuilder {
    pub async fn load(&self) -> Controller {
        let mut api_client = ApiClient::new();
        let db_client = DatabaseClient::new();

        let agent = db_client.load_agent(&self.callsign).await;
        let surveys_list = db_client.load_surveys(0).await;
        let surveys: HashMap<String, Vec<WrappedSurvey>> =
            surveys_list
                .into_iter()
                .fold(HashMap::new(), |mut acc, survey| {
                    let e = acc.entry(survey.inner().symbol.clone()).or_insert(vec![]);
                    e.push(survey);
                    acc
                });

        api_client.set_auth_token(agent.bearer_token.clone());

        Controller {
            api_client,
            db_client,
            agent,
            ships: HashMap::new(),
            markets: HashMap::new(),
            surveys: surveys,
        }
    }
}

pub struct Controller {
    pub api_client: ApiClient,
    pub db_client: DatabaseClient,

    // universe
    pub ships: HashMap<String, Ship>,
    pub markets: HashMap<String, Market>,
    pub agent: db_models::Agent,
    pub surveys: HashMap<String, Vec<WrappedSurvey>>,
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
            self.ships.insert(ship.symbol.clone(), ship);
        }
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

    pub async fn sleep_for_navigation(&mut self) {
        let ship = self.par.ships.get(&self.symbol).unwrap();
        // OutOfRangeError on negative duration
        if let Ok(duration) = (ship.nav.route.arrival - Utc::now()).to_std() {
            debug!(
                "Sleeping for navigation {}s",
                duration.as_millis() as f64 / 1000.0
            );
            tokio::time::sleep(duration).await;
        }
    }

    pub async fn sleep_for_cooldown(&mut self) {
        let ship = self.par.ships.get(&self.symbol).unwrap();
        if let Some(cooldown) = &ship.cooldown {
            // OutOfRangeError on negative duration
            if let Ok(duration) = (cooldown.expiration - Utc::now()).to_std() {
                debug!(
                    "Sleeping for cooldown {}s",
                    duration.as_millis() as f64 / 1000.0
                );
                tokio::time::sleep(duration).await;
            }
        }
    }

    pub async fn flight_mode(&mut self, target: &str) {
        let mut ship = self.par.ships.get_mut(&self.symbol).unwrap();
        if ship.nav.flight_mode == target {
            return;
        }
        debug!("Flight mode: {} -> {}", ship.nav.flight_mode, target);
        ship.nav = self.par.api_client.flight_mode(&self.symbol, target).await;
    }

    pub async fn orbit_status(&mut self, target: &str) {
        let mut ship = self.par.ships.get_mut(&self.symbol).unwrap();
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

    pub async fn navigate(&mut self, target: &str) {
        self.orbit_status("IN_ORBIT").await;
        let mut ship = self.par.ships.get_mut(&self.symbol).unwrap();
        if ship.nav.waypoint_symbol == target {
            return;
        }
        let (nav, fuel) = self.par.api_client.navigate(&self.symbol, target).await;
        ship.nav = nav;
        ship.fuel = fuel;
    }

    pub async fn fetch_market(&mut self) -> Market {
        let ship = self.par.ships.get(&self.symbol).unwrap();
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
            .insert(market.symbol.clone(), market.clone());
        market
    }

    pub async fn survey(&mut self) -> Vec<Survey> {
        self.orbit_status("IN_ORBIT").await;

        self.sleep_for_cooldown().await;

        let ship = self.par.ships.get_mut(&self.symbol).unwrap();
        let (surveys, cooldown) = self.par.api_client.survey(&ship.symbol).await;
        ship.cooldown = Some(cooldown);

        let wrapped: Vec<WrappedSurvey> = self.par.db_client.insert_surveys(&surveys).await;
        let e = self
            .par
            .surveys
            .entry(ship.nav.waypoint_symbol.clone())
            .or_insert(vec![]);
        e.extend(wrapped.clone());

        surveys
    }

    pub async fn extract_survey(&mut self, survey: &WrappedSurvey) {
        self.sleep_for_cooldown().await;

        let ship = self.par.ships.get_mut(&self.symbol).unwrap();
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
                if e.code == 4224 {
                    // depleted survey
                    debug!("Survey depleted, removing from database");
                    self.par.db_client.update_survey_state(&survey, 2).await;
                }
            }
        }
    }

    pub async fn refuel(&mut self) {
        let ship = self.par.ships.get_mut(&self.symbol).unwrap();
        let refuel_units = (ship.fuel.capacity - ship.fuel.current) / 100 * 100;
        if refuel_units == 0 {
            return;
        }
        debug!("Refuel: {} units", refuel_units);
        self.orbit_status("DOCKED").await;
        let (_agent, fuel) = self.par.api_client.refuel(&self.symbol, refuel_units).await;

        let ship = self.par.ships.get_mut(&self.symbol).unwrap();
        ship.fuel = fuel;
        debug!("Updated fuel: {:?}", ship.fuel.current);
    }

    pub async fn sell(&mut self, symbol: &str, units: u32) {
        self.orbit_status("DOCKED").await;
        let (_agent, cargo, t) = self.par.api_client.sell(&self.symbol, symbol, units).await;
        debug!("Sold {}x {}: +${}", t.units, t.trade_symbol, t.total_price);

        let ship = self.par.ships.get_mut(&self.symbol).unwrap();
        ship.cargo = cargo;
        debug!("Updated cargo: {:?}", ship.cargo);
    }
}
