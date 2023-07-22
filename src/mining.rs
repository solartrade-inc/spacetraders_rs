use core::panic;
use std::collections::{HashMap, HashSet};

use crate::decision_tree::{self, evaluate, Edge, Metric};
use crate::models::*;
use crate::{api_client::ApiClient, controller::Controller, database::DatabaseClient, util};
use graph_builder::{DirectedCsrGraph, GraphBuilder};
use lazy_static::lazy_static;
use log::debug;
use rand::prelude::*;
use rand::Rng;

pub struct PreparedGraph {
    pub nodes: HashMap<String, usize>,
    pub x0: f64,
    pub state: HashMap<String, decision_tree::State<String>>,
    pub edges: Vec<(String, String, Edge<Metric>)>,
    pub graph: DirectedCsrGraph<usize, (), Edge<Metric>>,
}

pub struct MiningExecutor {
    pub par: Controller,
    pub ship_idx: usize,
    pub asteroid_symbol: String,
    pub graph: PreparedGraph,
}
impl MiningExecutor {
    async fn run(&mut self) {
        loop {
            self.step().await;
            panic!("TODO");
        }
    }

    async fn step(&mut self) {
        // identify mining state
        let ship_symbol = format!("{}-{:x}", self.par.agent.symbol, self.ship_idx);
        let ship = self.par.ships.get_mut(&ship_symbol).unwrap().clone();

        // map S -> state
        // states: [D]start, [P]extract, [P]survey, [D]cargo_{symbol}, [D]cargo_{symbol}_stripped, [D]survey_{idx}, [P]extract_survey_{idx}, [D]finish

        let is_cargo_empty = ship.cargo.units == 0;
        let have_survey = false;

        let state: String = if ship.cargo.units == 0 {
            "start".into()
        } else {
            panic!("TODO");
        };
        debug!("Mining state: {}", state);
        let successor = &self.graph.state[&state].successor;
        debug!("Successor: {:?}", successor);
        match &successor.as_ref().map(|s| s.as_str()) {
            Some("survey") => {
                let mut ship_controller = self.par.ship_controller(self.ship_idx);
                ship_controller.survey().await;
            },
            _ => panic!("Unexpected successor: {:?}", successor),
        };
    }
}

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

    pub async fn run(mut self) {
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
        let asteroid_traits: Vec<String> = asteroid_waypoint
            .traits
            .iter()
            .map(|t| t.symbol.clone())
            .collect();

        // 2. load markets
        let mut markets: Vec<Market> = vec![];
        for waypoint in waypoints.iter() {
            if util::is_market(waypoint) {
                let market = self.par.db_client.load_market(&waypoint.symbol).await;
                markets.push(market);
            }
        }

        // let ship_mounts = vec![MOUNT_SURVEYOR_II.clone(), MINING_LASER_II.clone(), MINING_LASER_II.clone()];

        let g = Self::mining_prep(
            &asteroid_waypoint.symbol,
            &asteroid_traits,
            &markets,
            &ship.mounts,
        );
        debug!(
            "Full: {:?} cps over {} seconds",
            g.x0 - g.state["start"].fx.0 / g.state["start"].fx.1,
            -g.state["start"].fx.1
        );
        debug!(
            "Extract: {:?} cps over {} seconds",
            g.x0 - g.state["extract"].fx.0 / g.state["extract"].fx.1,
            -g.state["extract"].fx.1
        );
        debug!(
            "Survey: {:?} cps over {} seconds",
            g.x0 - g.state["survey"].fx.0 / g.state["survey"].fx.1,
            -g.state["survey"].fx.1
        );

        MiningExecutor {
            par: self.par,
            ship_idx: self.ship_idx,
            asteroid_symbol: self.asteroid_symbol.clone(),
            graph: g,
        }
        .run()
        .await;
    }

    pub fn mining_prep(
        asteroid_field_symbol: &str,
        asteroid_field_traits: &Vec<String>,
        markets: &Vec<Market>,
        ship_mounts: &Vec<ShipMount>,
    ) -> PreparedGraph {
        // construct decision tree

        let mut edges: Vec<(String, String, Edge<Metric>)> = vec![];

        let deposits = asteroid_yields(&asteroid_field_traits);
        let is_stripped = asteroid_field_traits.contains(&"STRIPPED".to_string());
        let _sum = deposits.values().sum::<usize>();

        debug!("Deposits: {:?}", deposits);

        let mut surveyor_cooldown: f64 = 60.0;
        let mut surveyors: Vec<_> = vec![];
        let mut extract_cooldown: f64 = 60.0;
        let mut mining_strength: f64 = 0.0;
        for mount in ship_mounts {
            if mount.symbol.starts_with("MOUNT_MINING_LASER_") {
                extract_cooldown += 10.0 * mount.requirements.power as f64;
                mining_strength += mount.strength.unwrap() as f64;
            }
            if mount.symbol.starts_with("MOUNT_SURVEYOR_") {
                surveyor_cooldown += 10.0 * mount.requirements.power as f64;
                let survey_deposits = mount.deposits.as_ref().unwrap();
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
        let surveys_per_operation = surveyors.iter().map(|(s, _)| *s).sum::<u32>();

        edges.push((
            "start".into(),
            "extract".into(),
            Edge::new_decision(Metric(0.0, 0.0)),
        ));
        // extract edges
        for (symbol, &weight) in deposits.iter() {
            let node = match is_stripped {
                true => format!("cargo_{}_stripped", symbol),
                false => format!("cargo_{}", symbol),
            };
            edges.push((
                "extract".into(),
                node,
                Edge::new_probability(Metric(0.0, extract_cooldown), weight as f64),
            ));
        }
        // sell + jettison edges
        for (&symbol, _weight) in deposits.iter() {
            let cargo_node = format!("cargo_{}", symbol);
            let cargo_node_stripped = format!("cargo_{}_stripped", symbol);
            // jettison
            edges.push((
                cargo_node.clone(),
                "finish".into(),
                Edge::new_decision(Metric(0.0, 0.0)),
            ));
            edges.push((
                cargo_node_stripped.clone(),
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
                    let mut profit_stripped = unit_sell_price as f64 * mining_strength / 2.0;
                    if market.symbol != asteroid_field_symbol {
                        duration += 10.0; // crude estimate of travel and return time
                        profit -= 50.0; // crude estimate of fuel cost
                        profit_stripped -= 50.0;
                    }
                    edges.push((
                        cargo_node.clone(),
                        "finish".into(),
                        Edge::new_decision(Metric(profit, duration)),
                    ));
                    edges.push((
                        cargo_node_stripped.clone(),
                        "finish".into(),
                        Edge::new_decision(Metric(profit_stripped, duration)),
                    ));
                }
            }
        }

        // survey edges
        edges.push((
            "start".into(),
            "survey".into(),
            Edge::new_decision(Metric(0.0, 0.0)),
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
                            .choose_weighted(&mut rand::thread_rng(), |symbol| {
                                YIELD_WEIGHTS[symbol.as_str()]
                            })
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
            let duration = surveyor_cooldown / (surveys_per_operation as f64);
            edges.push((
                "survey".into(),
                survey_node.clone(),
                Edge::new_probability(Metric(0.0, duration), 1.0),
            ));
            edges.push((
                survey_node.clone(),
                "finish".into(),
                Edge::new_decision(Metric(0.0, 0.0)),
            ));
            edges.push((
                survey_node.clone(),
                extract_survey_node.clone(),
                Edge::new_repeatable_decision(Metric(0.0, 0.0), 15),
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
            let mut nodes: HashMap<String, usize> = HashMap::new();
            let mut nodes_inv: Vec<String> = vec![];
            nodes.insert("start".into(), 0);
            nodes_inv.push("start".into());
            for (from, to, ref edge) in edges.iter() {
                let from_idx = match nodes.get(from) {
                    Some(&idx) => idx,
                    None => {
                        let len = nodes.len();
                        nodes.insert(from.into(), len);
                        nodes_inv.push(from.into());
                        len
                    }
                };
                let to_idx = match nodes.get(to) {
                    Some(&idx) => idx,
                    None => {
                        let len = nodes.len();
                        nodes.insert(to.into(), len);
                        nodes_inv.push(to.into());
                        len
                    }
                };
                edges1.push((from_idx, to_idx, edge.clone()));
            }

            let graph: DirectedCsrGraph<usize, (), Edge<Metric>> =
                GraphBuilder::new().edges_with_values(edges1).build();
            let g1 = evaluate(&graph, nodes["start"]);
            
            let mut g = HashMap::new();
            for (node_name, node_idx) in nodes.iter() {
                if let Some(&ref entry) = g1.1.get(&node_idx) {
                    let entry1 = decision_tree::State {
                        fx: entry.fx,
                        successor: entry.successor.map(|s| nodes_inv[s].clone()),
                    };
                    g.insert(node_name.to_string(), entry1);
                }
            }
            PreparedGraph {
                nodes: nodes,
                x0: g1.0,
                state: g,
                edges,
                graph: graph,
            }
        }
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
    static ref BASE_DEPOSITS: Vec<String> = vec!["QUARTZ_SAND".into(), "SILICON_CRYSTALS".into(), "PRECIOUS_STONES".into(), "ICE_WATER".into(), "AMMONIA_ICE".into(), "IRON_ORE".into(), "COPPER_ORE".into(), "SILVER_ORE".into(), "ALUMINUM_ORE".into(), "GOLD_ORE".into(), "PLATINUM_ORE".into()];

    static ref MOUNT_SURVEYOR_I: ShipMount = ShipMount { symbol: "MOUNT_SURVEYOR_I".into(), strength: Some(1),
        deposits: Some(BASE_DEPOSITS.clone()),
        requirements: ShipMountRequirements { power: 1, crew: 2, slots: None } };
    static ref MOUNT_SURVEYOR_II: ShipMount = {
        let mut surveyor = ShipMount { symbol: "MOUNT_SURVEYOR_II".into(), strength: Some(2),
            deposits: Some(BASE_DEPOSITS.clone()),
            requirements: ShipMountRequirements { power: 4, crew: 3, slots: None } };
        surveyor.deposits.as_mut().unwrap().push("DIAMONDS".into());
        surveyor.deposits.as_mut().unwrap().push("URANITE_ORE".into());
        surveyor
    };
    static ref MOUNT_SURVEYOR_III: ShipMount = {
        let mut surveyor = ShipMount { symbol: "MOUNT_SURVEYOR_III".into(), strength: Some(3),
            deposits: Some(BASE_DEPOSITS.clone()),
            requirements: ShipMountRequirements { power: 7, crew: 5, slots: None } };
        surveyor.deposits.as_mut().unwrap().push("DIAMONDS".into());
        surveyor.deposits.as_mut().unwrap().push("MERITIUM_ORE".into());
        surveyor
    };

    static ref MINING_LASER_II: ShipMount = ShipMount { symbol: "MOUNT_MINING_LASER_II".into(), strength: Some(25), deposits: None, requirements: ShipMountRequirements { power: 2, crew: 2, slots: None }};

    static ref YIELD_WEIGHTS: HashMap<&'static str, usize> = {
        HashMap::from([
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
        ])
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
