use std::collections::{HashMap, HashSet};

use crate::decision_tree::{self, evaluate, Edge, Metric};
use crate::models::*;
use crate::{api_client::ApiClient, controller::Controller, database::DatabaseClient, util};
use graph_builder::{DirectedCsrGraph, GraphBuilder};
use lazy_static::lazy_static;
use log::debug;
use rand::Rng;
use rand::prelude::*;

pub struct MiningController {
    pub par: Controller,
    pub ship_idx: i32,
    pub asteroid_symbol: String,
}

impl MiningController {
    pub fn new(par: Controller, ship_idx: i32, asteroid_symbol: String) -> Self {
        Self {
            par,
            ship_idx,
            asteroid_symbol,
        }
    }

    pub async fn run(&mut self) {
        // 0. load ship
        let ship_symbol = format!("{}-{:x}", self.par.agent.symbol, self.ship_idx);
        let ship = self.par.ships.get_mut(&ship_symbol).unwrap().clone();

        // 1. load asteroid
        let ship_system = ship.nav.system_symbol.clone();
        let waypoints = self.par.fetch_system_waypoints(&ship_system).await;
        let asteroid_waypoint = waypoints
            .iter()
            .find(|w| w.symbol == self.asteroid_symbol)
            .unwrap();

        // 2. load markets
        let mut markets: Vec<Market> = vec![];
        for waypoint in waypoints.iter() {
            if util::is_market(waypoint) {
                let market = self.par.db_client.load_market(&waypoint.symbol).await;
                markets.push(market);
            }
        }
        let asteroid_market = markets
            .iter()
            .find(|m| m.symbol == asteroid_waypoint.symbol)
            .unwrap();

        debug!("Ship: {:?}", ship);
        debug!("Asteroid: {:?}", asteroid_waypoint.traits);
        debug!("Markets: {:?}", markets);

        // construct decision tree based on:

        let mut edges: Vec<(String, String, Edge<Metric>)> = vec![];

        let traits: Vec<String> = asteroid_waypoint
            .traits
            .iter()
            .map(|t| t.symbol.clone())
            .collect();
        let deposits = asteroid_yields(&traits);
        let _sum = deposits.values().sum::<usize>();

        debug!("Deposits: {:?}", deposits);

        let mut surveyor_cooldown: f64 = 60.0;
        let mut surveyors: Vec<_> = vec![];
        let mut extract_cooldown: f64 = 60.0;
        let mut mining_strength: f64 = 0.0;
        for mount in ship.mounts {
            if mount.symbol.starts_with("MOUNT_MINING_LASER_") {
                extract_cooldown += 10.0 * mount.requirements.power as f64;
                mining_strength += mount.strength.unwrap() as f64;
            }
            if mount.symbol.starts_with("MOUNT_SURVEYOR_") {
                surveyor_cooldown += 10.0 * mount.requirements.power as f64;
                let survey_deposits = mount.deposits.unwrap();
                // calculate intersection of deposits and survey_deposits
                let mut intersection: Vec<String> = Vec::new();
                for (&symbol, &_weight) in deposits.iter() {
                    if survey_deposits.contains(&symbol.to_string()) {
                        intersection.push(symbol.to_string());
                    }
                }
                surveyors.push((mount.strength.unwrap(), intersection));
            }
        }

        edges.push((
            "start".into(),
            "extract".into(),
            Edge::new_decision(Metric(0.0, 0.0)),
        ));
        // extract edges
        for (symbol, &weight) in deposits.iter() {
            let node = format!("cargo_{}", symbol);
            edges.push((
                "extract".into(),
                node,
                Edge::new_probability(Metric(0.0, extract_cooldown), weight as f64),
            ));
        }
        // sell + jettison edges
        for (&symbol, _weight) in deposits.iter() {
            let cargo_node = format!("cargo_{}", symbol);
            // jettison
            edges.push((
                cargo_node.clone(),
                "finish".into(),
                Edge::new_decision(Metric(0.0, 0.0)),
            ));

            // sell
            for market in markets.iter() {
                let sell_price = market
                    .trade_goods
                    .iter()
                    .find(|g| g.symbol == symbol)
                    .map(|g| g.sell_price);
                if let Some(unit_sell_price) = sell_price {
                    let mut duration = 0.0;
                    let mut profit = unit_sell_price as f64 * mining_strength;
                    if market.symbol != asteroid_market.symbol {
                        duration += 10.0; // crude estimate of travel and return time
                        profit -= 50.0; // crude estimate of fuel cost
                    }
                    edges.push((
                        cargo_node.clone(),
                        "finish".into(),
                        Edge::new_decision(Metric(profit, duration)),
                    ));
                }
            }
        }

        // survey edges
        edges.push((
            "start".into(),
            "survey".into(),
            Edge::new_decision(Metric(0.0, 1.0)),
        ));

        // for the probability edges, there are too many combinations to fully enumerate,
        // so we'll generate a sample of 10k, and that should be good enough to accurately calculate rate,
        // and therefore gives us a decision model for the surveys that we didn't explicitly enumerate

        let mut sample_surveys = vec![];
        loop {
            for (strength, deposits) in surveyors.iter() {
                for _ in 0..*strength {
                    let num_deposits = rand::thread_rng().gen_range(3..=7);
                    let mut survey = vec![];
                    for _ in 0..num_deposits {
                        let deposit = deposits
                            .choose_weighted(&mut rand::thread_rng(), |symbol| YIELD_WEIGHTS[symbol.as_str()])
                            .unwrap();
                        survey.push(deposit.clone());
                    }
                    sample_surveys.push(survey);
                }
            }

            if sample_surveys.len() >= 10_000 {
                break;
            }
        }

        for (survey_idx, survey) in sample_surveys.iter().enumerate() {
            let survey_node = format!("survey_{}", survey_idx);
            let extract_survey_node = format!("extract_survey_{}", survey_idx);
            edges.push((
                "survey".into(),
                survey_node.clone(),
                Edge::new_probability(Metric(0.0, surveyor_cooldown), 1.0),
            ));
            edges.push((
                survey_node.clone(),
                "finish".into(),
                Edge::new_decision(Metric(0.0, 0.0)),
            ));
            edges.push((
                survey_node.clone(),
                extract_survey_node.clone(),
                Edge::new_decision(Metric(0.0, 0.0)),
            ));
            for deposit in survey.iter() {
                edges.push((
                    extract_survey_node.clone(),
                    format!("cargo_{}", deposit),
                    Edge::new_probability(Metric(0.0, extract_cooldown), 1.0),
                ));
            }
        }

        {
            let mut edges1: Vec<(usize, usize, Edge<Metric>)> = vec![];
            let mut nodes: HashMap<&str, usize> = HashMap::new();
            nodes.insert("start", 0);
            for (i, &(ref from, ref to, ref edge)) in edges.iter().enumerate() {
                let from_idx = *nodes.entry(from.as_str()).or_insert(i);
                let to_idx = *nodes.entry(to.as_str()).or_insert(2*edges.len() + i);
                edges1.push((from_idx, to_idx, edge.clone()));
            }

            let graph: DirectedCsrGraph<usize, (), Edge<Metric>> =
                GraphBuilder::new().edges_with_values(edges1).build();

            evaluate(&graph);
        }

        // go to the asteroid and do some mining
    }
}

// get yields for a given set of traits
fn asteroid_yields(traits: &Vec<String>) -> HashMap<&'static str, usize> {
    let mut s = HashSet::new();
    for trait_name in traits.iter() {
        let yields = TRAIT_YIELDS.get(trait_name.as_str());
        if let Some(yields) = yields {
            for &symbol in yields.iter() {
                s.insert(symbol);
            }
        } else {
            debug!("No yields for trait: {}", trait_name);
        }
    }
    let mut m = HashMap::new();
    for &symbol in s.iter() {
        let weight = YIELD_WEIGHTS.get(symbol).unwrap();
        m.insert(symbol, *weight);
    }
    m
}

lazy_static::lazy_static! {
    static ref YIELD_WEIGHTS: HashMap<&'static str, usize> = {
        let m = HashMap::from([
            ("ICE_WATER", 200),

            ("SILICON_CRYSTALS", 100),
            ("AMMONIA_ICE", 100),
            ("QUARTZ_SAND", 100),
            ("LIQUID_NITROGEN", 100),
            ("LIQUID_HYDROGEN", 100),

            ("HYDROCARBON", 50),
            ("IRON_ORE", 50),
            ("ALUMINUM_ORE", 50),
            ("COPPER_ORE", 50),
            ("SILVER_ORE", 50),
            ("PRECIOUS_STONES", 50),

            ("GOLD_ORE", 20),
            ("PLATINUM_ORE", 20),
            ("URANITE_ORE", 20),

            ("MERITIUM_ORE", 5),

            ("DIAMONDS", 1),
        ]);
        m
    };

    static ref TRAIT_YIELDS: HashMap<&'static str, Vec<&'static str>> = {
        let mut m = HashMap::new();
        m.insert("MINERAL_DEPOSITS", vec![
            "ICE_WATER",
            "QUARTZ_SAND",
            "SILICON_CRYSTALS",
            "AMMONIA_ICE",
            "IRON_ORE",
            "PRECIOUS_STONES",
            "DIAMONDS",
        ]);
        m.insert("ICE_CRYSTALS", vec![
            "ICE_WATER",
        ]);
        m.insert("COMMON_METAL_DEPOSITS", vec![
            "ICE_WATER",
            "QUARTZ_SAND",
            "SILICON_CRYSTALS",
            "IRON_ORE",
            "COPPER_ORE",
            "ALUMINUM_ORE",
        ]);
        m.insert("PRECIOUS_METAL_DEPOSITS", vec![
            "ICE_WATER",
            "QUARTZ_SAND",
            "SILICON_CRYSTALS",
            "IRON_ORE",
            "COPPER_ORE",
            "ALUMINUM_ORE",
            "SILVER_ORE",
            "GOLD_ORE",
            "PLATINUM_ORE",
        ]);
        m.insert("RARE_METAL_DEPOSITS", vec![
            "ICE_WATER",
            "QUARTZ_SAND",
            "SILICON_CRYSTALS",
            "COPPER_ORE",
            "ALUMINUM_ORE",
            "GOLD_ORE",
            "PLATINUM_ORE",
            "URANITE_ORE",
            "MERITIUM_ORE",
        ]);
        m.insert("METHANE_POOLS", vec![
            "HYDROCARBON",
        ]);
        m.insert("EXPLOSIVE_GASES", vec![
            "HYDROCARBON",
            "LIQUID_NITROGEN",
            "LIQUID_HYDROGEN",
        ]);
        m
    };
}
