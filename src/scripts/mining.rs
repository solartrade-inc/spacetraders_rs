use crate::decision_tree::{self, evaluate, Edge, EdgeType, Metric};
use crate::models::*;
use crate::runtime::Step;
use crate::{controller::Controller, util};
use async_trait::async_trait;
use core::panic;
use graph_builder::{DirectedCsrGraph, GraphBuilder};
use log::debug;
use rand::prelude::*;
use rand::Rng;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock as AsyncRwLock;

const EXPECTED_NUM_EXTRACTS: u32 = 10;

pub struct PreparedGraph {
    pub nodes: HashMap<String, usize>,
    pub x0: f64,
    pub state: HashMap<String, decision_tree::State<String>>,
    pub edges: Vec<(String, String, Edge<Metric>)>,
    pub graph: DirectedCsrGraph<usize, (), Edge<Metric>>,
}

pub struct MiningExecutor {
    pub par: Controller,
    pub ship_symbol: String,
    pub ship_arc: Arc<AsyncRwLock<Ship>>,
    pub asteroid_symbol: String,
    pub graph: PreparedGraph,
}
impl MiningExecutor {
    fn judge(&self, survey: &Survey) -> bool {
        use graph_builder::DirectedNeighborsWithValues as _;

        // we are at a transient decision node in the decision tree like survey_x, which leads to extract_survey_x, or discard_survey_x
        // extract_survey_x is a transient probability node which leads to cargo_{symbol} for each deposit

        // steal the 'extract' duration weight
        let extract_survey_0_idx = self.graph.nodes["extract_survey_0"];
        let example_edge = self
            .graph
            .graph
            .out_neighbors_with_values(extract_survey_0_idx)
            .next()
            .unwrap()
            .value;
        let extract_duration = example_edge.metric.1;

        let (f_b, df_b) = {
            // extract_survey_x
            let mut sum = (0.0, 0.0);
            let mut weight_sum = 0.0;
            for t in survey.deposits.iter() {
                let y = format!("cargo_{}", &t.symbol);
                let edge = Metric(0.0, extract_duration);
                let edge_weight = 1.0;
                let (g, dg) = self.graph.state[&y].fx;
                let f = g + (edge.0 - self.graph.x0 * edge.1);
                let df = dg - edge.1;
                sum.0 += f * edge_weight;
                sum.1 += df * edge_weight;
                weight_sum += edge_weight;
            }
            (sum.0 / weight_sum, sum.1 / weight_sum)
        };
        // debug!("fB: {:?} dfB: {:?}", f_b, df_b);
        debug!(
            "Survey judge: {:?} cps over {} seconds",
            self.graph.x0 - f_b / df_b,
            -df_b
        );

        let mut successor = None;
        let (_f_a, _df_a) = {
            let edges = vec![
                (
                    Edge::new_repeatable_decision(Metric(0.0, 0.0), EXPECTED_NUM_EXTRACTS),
                    (f_b, df_b),
                ),
                (
                    Edge::new_decision(Metric(0.0, 0.0)),
                    self.graph.state["finish"].fx,
                ),
            ];

            let mut max = (f64::MIN, f64::MIN);
            for (idx, &t) in edges.iter().enumerate() {
                let edge = t.0.metric;
                let repeats = match t.0.edge_type {
                    EdgeType::Decision(repeats) => repeats,
                    _ => panic!(),
                } as f64;
                let (g, dg) = t.1;
                let f = repeats * g + (edge.0 - self.graph.x0 * edge.1);
                let df = repeats * dg - edge.1;
                if f > max.0 || f == max.0 && df > max.1 {
                    max.0 = f;
                    max.1 = df;
                    successor = Some(idx);
                }
            }
            max
        };
        // debug!("fA: {:?} dfA: {:?} successor: {:?}", fA, dfA, successor);
        match successor.unwrap() {
            0 => true,
            1 => false,
            _ => panic!(),
        }
    }
}

#[async_trait]
impl Step for MiningExecutor {
    async fn step(&self) -> Option<Duration> {
        // identify mining state
        let ship = self.ship_arc.read().await;

        // Work out our current state at the start of the step
        let is_cargo_empty = ship.cargo.units == 0;
        let mut usable_surveys = vec![];

        let state: String = if is_cargo_empty {
            let surveys: Vec<Arc<WrappedSurvey>> = self
                .par
                .surveys
                .entry(self.asteroid_symbol.clone())
                .or_insert(vec![])
                .clone();
            for survey in surveys.iter() {
                if survey.inner().expiration < chrono::Utc::now() {
                    continue;
                }
                let usuable = self.judge(survey.inner());
                if usuable {
                    usable_surveys.push(survey.clone());
                }
            }
            debug!(
                "Surveys: {} usuable of {}",
                usable_surveys.len(),
                surveys.len()
            );
            if !usable_surveys.is_empty() {
                "survey_x".into()
            } else {
                "start".into()
            }
        } else {
            debug!("Holding cargo: {:?}", ship.cargo);
            let item = &ship.cargo.inventory[0];
            if item.units >= 20 {
                // @@ should be tied to mining laser strength
                format!("cargo_{}", item.symbol)
            } else {
                format!("cargo_{}_stripped", item.symbol)
            }
        };
        debug!("Mining state: {}", state);

        let successor = match state.as_str() {
            "survey_x" => Some("extract_survey_x".into()),
            _ => self.graph.state[&state].successor.clone(),
        };
        drop(ship);

        debug!("Successor: {:?}", successor);
        match &successor.as_deref() {
            Some("survey") => {
                let mut ship_controller = self.par.ship_controller(&self.ship_symbol).await;
                ship_controller.navigate(&self.asteroid_symbol).await;
                if let Some(cooldown) = ship_controller.navigation_cooldown() {
                    return Some(cooldown);
                }
                if let Some(cooldown) = ship_controller.reactor_cooldown() {
                    return Some(cooldown);
                }
                ship_controller.survey().await;
            }
            Some("extract_survey_x") => {
                let mut ship_controller = self.par.ship_controller(&self.ship_symbol).await;
                ship_controller.navigate(&self.asteroid_symbol).await;
                if let Some(cooldown) = ship_controller.navigation_cooldown() {
                    return Some(cooldown);
                }
                if let Some(cooldown) = ship_controller.reactor_cooldown() {
                    return Some(cooldown);
                }
                ship_controller.extract_survey(&usable_surveys[0]).await;
            }
            Some(s) => {
                lazy_static::lazy_static!(
                    static ref SELL_REGEX: Regex = Regex::new(r"^sell_(?P<market>.+)$").unwrap();
                    static ref CARGO_REGEX: Regex = Regex::new(r"^cargo_(?P<symbol>.+)$").unwrap();
                );
                // check if s matches sell regex:
                if let Some(captures) = SELL_REGEX.captures(s) {
                    let market_symbol = captures.name("market").unwrap().as_str();
                    let mut ship_controller = self.par.ship_controller(&self.ship_symbol).await;
                    ship_controller.navigate(market_symbol).await;
                    if let Some(cooldown) = ship_controller.navigation_cooldown() {
                        return Some(cooldown);
                    }
                    let item = ship_controller.ship.cargo.inventory[0].clone();
                    ship_controller.sell(&item.symbol, item.units).await;
                } else {
                    panic!("Unexpected successor: {:?}", successor);
                }
            }
            None => {
                panic!("Unexpected successor: {:?}", successor);
            }
        };
        Some(Duration::from_secs(0))
    }
}

pub struct MiningController {
    pub par: Controller,
    ship_arc: Arc<AsyncRwLock<Ship>>,
    pub asteroid_symbol: String,
}

impl MiningController {
    pub fn new(par: &Controller, ship_symbol: &str, asteroid_symbol: &str) -> Self {
        Self {
            par: par.clone(),
            ship_arc: par.ships.get(ship_symbol).unwrap().clone(),
            asteroid_symbol: asteroid_symbol.into(),
        }
    }

    pub async fn setup(self) -> MiningExecutor {
        let ship = self.ship_arc.read().await;

        // 1. load asteroid
        let ship_system = util::system_symbol(&self.asteroid_symbol);
        // @@ should read systems and waypoints from memory, not from api
        let waypoints = self
            .par
            .api_client
            .fetch_system_waypoints(&ship_system)
            .await;
        let asteroid_waypoint = waypoints
            .iter()
            .find(|w| w.symbol == self.asteroid_symbol)
            .unwrap();
        let asteroid_traits: Vec<String> = asteroid_waypoint
            .traits
            .iter()
            .map(|t| t.symbol.clone())
            .collect();
        debug!("Mounts: {:?}", ship.mounts);

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

        let e = MiningExecutor {
            par: self.par.clone(),
            ship_symbol: ship.symbol.clone(),
            ship_arc: self.ship_arc.clone(),
            asteroid_symbol: self.asteroid_symbol.clone(),
            graph: g,
        };
        drop(ship);
        e
    }

    pub fn mining_prep(
        asteroid_field_symbol: &str,
        asteroid_field_traits: &Vec<String>,
        markets: &Vec<Market>,
        ship_mounts: &Vec<ShipMount>,
    ) -> PreparedGraph {
        // construct decision tree

        let mut edges: Vec<(String, String, Edge<Metric>)> = vec![];

        let deposits = asteroid_yields(asteroid_field_traits);
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
        let surveys_per_operation = surveyors.iter().map(|(strength, _)| *strength).sum::<u32>();

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
                let sell_node = format!("sell_{}", market.symbol);
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
                        sell_node.clone(),
                        Edge::new_decision(Metric(profit, duration)),
                    ));
                    edges.push((
                        cargo_node_stripped.clone(),
                        sell_node.clone(),
                        Edge::new_decision(Metric(profit_stripped, duration)),
                    ));
                }
                // mark sell_node as a terminal node
                edges.push((
                    sell_node,
                    "finish".into(),
                    Edge::new_decision(Metric(0.0, 0.0)),
                ));
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
                Edge::new_repeatable_decision(Metric(0.0, 0.0), EXPECTED_NUM_EXTRACTS),
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
                edges1.push((from_idx, to_idx, *edge));
            }

            let graph: DirectedCsrGraph<usize, (), Edge<Metric>> =
                GraphBuilder::new().edges_with_values(edges1).build();
            let g1 = evaluate(&graph, nodes["start"]);

            let mut g = HashMap::new();
            for (node_name, node_idx) in nodes.iter() {
                if let Some(entry) = g1.1.get(node_idx) {
                    let entry1 = decision_tree::State {
                        fx: entry.fx,
                        successor: entry.successor.map(|s| nodes_inv[s].clone()),
                    };
                    g.insert(node_name.to_string(), entry1);
                }
            }
            PreparedGraph {
                nodes,
                x0: g1.0,
                state: g,
                edges,
                graph,
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

    static ref MOUNT_SURVEYOR_I: ShipMount = ShipMount {
        symbol: "MOUNT_SURVEYOR_I".into(), strength: Some(1),
        deposits: Some(BASE_DEPOSITS.clone()),
        requirements: ShipMountRequirements { power: 1, crew: 2, slots: None } };
    static ref MOUNT_SURVEYOR_II: ShipMount = {
        let mut surveyor = ShipMount {
            symbol: "MOUNT_SURVEYOR_II".into(), strength: Some(2),
            deposits: Some(BASE_DEPOSITS.clone()),
            requirements: ShipMountRequirements { power: 4, crew: 3, slots: None } };
        surveyor.deposits.as_mut().unwrap().push("DIAMONDS".into());
        surveyor.deposits.as_mut().unwrap().push("URANITE_ORE".into());
        surveyor
    };
    static ref MOUNT_SURVEYOR_III: ShipMount = {
        let mut surveyor = ShipMount {
            symbol: "MOUNT_SURVEYOR_III".into(), strength: Some(3),
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
