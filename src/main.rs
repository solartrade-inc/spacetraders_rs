use pathfinding::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    time::Instant,
    vec::Vec,
};

#[derive(Serialize, Deserialize, Debug)]
struct Trait {
    name: String,
    description: String,
    symbol: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Waypoint {
    symbol: String,
    x: i32,
    y: i32,
    #[serde(rename = "type")]
    waypoint_type: String,
    traits: Option<Vec<Trait>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct System {
    symbol: String,
    #[serde(rename = "type")]
    system_type: String,
    x: i32,
    y: i32,
    waypoints: Vec<Waypoint>,
}

fn main() {
    let json = std::fs::read_to_string("charted_systems.json").unwrap();
    let charted_systems: Vec<System> = serde_json::from_str(&json).unwrap();

    // nodes are:
    // add every market and jumpgate waypoint node
    // every remaining system is added a a system node
    L2(&charted_systems);
}

fn dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    ((((a.0 - b.0) as i64).pow(2) + ((a.1 - b.1) as i64).pow(2)) as f64)
        .sqrt()
        .round() as i32
}

const MAX_FUEL: i32 = 1200;
const SHIP_SPEED: i32 = 30;
const MAX_JUMPGATE: i32 = 2000;
const MAX_WARP: i32 = 10000;

fn L2(charted_systems: &Vec<System>) {
    #[derive(PartialEq)]
    struct SystemPoint(i32, i32);
    #[derive(PartialEq)]
    struct WaypointPoint(i32, i32);
    #[derive(PartialEq)]
    enum Node {
        JumpgateWaypoint(SystemPoint, WaypointPoint),
        MarketWaypoint(SystemPoint, WaypointPoint),
        System(SystemPoint),
    }
    use Node::*;
    #[derive(Clone, Debug)]
    enum FlightMode {
        CRUISE,
        BURN,
        DRIFT,
    }
    use FlightMode::*;
    #[derive(Clone, Debug)]
    enum Edge {
        Jumpgate,
        Warp(FlightMode),
        Nav(FlightMode),
    }

    let mut l2_nodes: Vec<Node> = vec![];
    let mut l2_nodes_name: Vec<String> = vec![];
    for system in charted_systems {
        let mut added = 0;
        for waypoint in &system.waypoints {
            let is_market = match &waypoint.traits {
                Some(traits) => traits.iter().any(|t| t.symbol == "MARKETPLACE"),
                None => false,
            };
            if waypoint.waypoint_type == "JUMP_GATE" {
                l2_nodes.push(Node::JumpgateWaypoint(
                    SystemPoint(system.x, system.y),
                    WaypointPoint(waypoint.x, waypoint.y),
                ));
                l2_nodes_name.push(waypoint.symbol.clone());
                added += 1;
            }
            else if is_market || waypoint.symbol == "X1-FT59-41745B" || waypoint.symbol == "X1-QP42-01002A" {
                l2_nodes.push(Node::MarketWaypoint(
                    SystemPoint(system.x, system.y),
                    WaypointPoint(waypoint.x, waypoint.y),
                ));
                l2_nodes_name.push(waypoint.symbol.clone());
                added += 1;
            }
        }
        if added == 0 && system.waypoints.len() != 0 {
            l2_nodes.push(Node::System(SystemPoint(system.x, system.y)));
            l2_nodes_name.push(system.symbol.clone())
        }
    }

    println!("{:#?}", charted_systems.len());
    // count each enum type
    let mut jumpgate_waypoints = 0;
    let mut market_waypoints = 0;
    let mut systems_nodes = 0;
    for node in &l2_nodes {
        match node {
            JumpgateWaypoint(_, _) => jumpgate_waypoints += 1,
            MarketWaypoint(_, _) => market_waypoints += 1,
            System(_) => systems_nodes += 1,
        }
    }
    println!(
        "jump: {:#?} market: {:#?} system: {:#?} total: {:#?}",
        jumpgate_waypoints,
        market_waypoints,
        systems_nodes,
        l2_nodes.len()
    );

    let mut l2_adj = vec![vec![]; l2_nodes.len()];
    for (i, node_i) in l2_nodes.iter().enumerate() {
        for (j, node_j) in l2_nodes.iter().enumerate() {
            // J <-> J: jump
            // * <-> * (same system): nav (B, C, D)
            // * <-> * (different system): warp (B, C, D)
            if i == j {
                continue;
            }
            if let (JumpgateWaypoint(j1, _), JumpgateWaypoint(j2, _)) = &(node_i, node_j) {
                let distance = dist((j1.0, j1.1), (j2.0, j2.1));
                if distance <= MAX_JUMPGATE {
                    let duration = max(60, ((distance as f64) / 10f64).round() as i32);
                    l2_adj[i].push((j, Edge::Jumpgate, duration, 0));
                }
            }

            let sys_i = match node_i {
                JumpgateWaypoint(SystemPoint(x, y), _) => SystemPoint(*x, *y),
                MarketWaypoint(SystemPoint(x, y), _) => SystemPoint(*x, *y),
                System(SystemPoint(x, y)) => SystemPoint(*x, *y),
            };
            let sys_j = match node_j {
                JumpgateWaypoint(SystemPoint(x, y), _) => SystemPoint(*x, *y),
                MarketWaypoint(SystemPoint(x, y), _) => SystemPoint(*x, *y),
                System(SystemPoint(x, y)) => SystemPoint(*x, *y),
            };
            let sys_dist = dist((sys_i.0, sys_i.1), (sys_j.0, sys_j.1));
            if sys_dist == 0 {
                // nav
                let nav_i = match node_i {
                    JumpgateWaypoint(_, WaypointPoint(x, y)) => WaypointPoint(*x, *y),
                    MarketWaypoint(_, WaypointPoint(x, y)) => WaypointPoint(*x, *y),
                    System(_) => panic!("shouldn't happen"),
                };
                let nav_j = match node_j {
                    JumpgateWaypoint(_, WaypointPoint(x, y)) => WaypointPoint(*x, *y),
                    MarketWaypoint(_, WaypointPoint(x, y)) => WaypointPoint(*x, *y),
                    System(_) => panic!("shouldn't happen"),
                };
                let nav_dist = dist((nav_i.0, nav_i.1), (nav_j.0, nav_j.1));
                // nav: CRUISE
                {
                    let fuel = nav_dist;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 1. / 15.;
                    let duration = 15 + (nav_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL {
                        l2_adj[i].push((j, Edge::Nav(CRUISE), duration, fuel));
                    }
                }
                // nav BURN
                {
                    let fuel = nav_dist * 2;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 2. / 15.;
                    let duration = 15 + (nav_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL {
                        l2_adj[i].push((j, Edge::Nav(BURN), duration, fuel));
                    }
                }
                // nav DRIFT
                {
                    let fuel = 1;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 0.1 / 15.;
                    let duration = 15 + (nav_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL {
                        l2_adj[i].push((j, Edge::Nav(DRIFT), duration, fuel));
                    }
                }
            } else {
                // warp: BURN
                {
                    let fuel = sys_dist * 2;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 2. / 20.;
                    let duration = 15 + (sys_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL {
                        l2_adj[i].push((j, Edge::Warp(BURN), duration, fuel));
                    }
                }
                // warp: CRUISE
                {
                    let fuel = sys_dist;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 1. / 20.;
                    let duration = 15 + (sys_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL {
                        l2_adj[i].push((j, Edge::Warp(CRUISE), duration, fuel));
                    }
                }
                // warp: DRIFT
                {
                    let fuel = 1;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 0.1 / 20.;
                    let duration = 15 + (sys_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL && sys_dist <= MAX_WARP {
                        l2_adj[i].push((j, Edge::Warp(DRIFT), duration, fuel));
                    }
                }
            }
        }
    }

    let num_edges = l2_adj.iter().map(|x| x.len()).sum::<usize>();
    println!("num edges: {}", num_edges);

    let COSMIC_HQ = MarketWaypoint(SystemPoint(-9804, -10050), WaypointPoint(-13, 18)); // X1-ZT91-90060F
    let ANCIENTS_HQ = MarketWaypoint(SystemPoint(-48384, 28029), WaypointPoint(26, -3)); // X1-QM50-15330F
    let CORSAIRS_HQ = MarketWaypoint(SystemPoint(-30024, -29491), WaypointPoint(1, 26)); // X1-XR77-94090F
    let OBSIDIAN_HQ = MarketWaypoint(SystemPoint(-9614, -30237), WaypointPoint(23, -12)); // X1-GX98-61300D

    let LORDS = "X1-QM47-80470D";
    let SOLITARY = "X1-RS97-03910B";
    let ECHO = "X1-QN84-21330Z";
    let DOMINION = "X1-MZ97-82310B";
    let OBSIDIAN = "X1-GX98-61300D";
    let COSMIC = "X1-ZT91-90060F";

    let src = l2_nodes_name
        .iter()
        .position(|x| x == "X1-FT59-41745B")
        .unwrap();
    let dest = l2_nodes_name
        .iter()
        .position(|x| x == "X1-QP42-01002A")
        .unwrap();

    let start = Instant::now();
    let result = astar::<usize, i32, _, _, _, _>(
        &src,
        |&n| l2_adj[n].iter().map(|&(e, _, w, _f)| (e, w)),
        |_| 0,
        |&n| n == dest,
    );
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);

    let (path, duration) = result.unwrap();
    let names: Vec<&str> = path.iter().map(|&i| l2_nodes_name[i].as_str()).collect();

    println!(
        "Planned route from {} to {} under fuel constraint: {MAX_FUEL}",
        l2_nodes_name[src], l2_nodes_name[dest]
    );
    for i in 0..path.len() - 1 {
        let edge = l2_adj[path[i]]
            .iter()
            .find(|&&(j, _, _, _)| j == path[i + 1])
            .unwrap();
        if let MarketWaypoint(..) = l2_nodes[path[i]] {
            println!("Refuel\t\t{}", names[i]);
        }
        println!(
            "{:?}\t{:14}  ->  {:14}\t{}s\t{}",
            edge.1,
            names[i],
            names[i + 1],
            edge.2,
            edge.3,
        );
    }

    println!("Total duration: {}s", duration);
}

fn L1(charted_systems: &Vec<System>) {
    let mut l1_nodes = vec![];
    for system in charted_systems {
        for waypoint in &system.waypoints {
            if waypoint.waypoint_type == "JUMP_GATE" {
                l1_nodes.push((system.x, system.y));
            }
        }
    }
    println!("{:#?}", charted_systems.len());
    println!("{:#?}", l1_nodes.len());

    let mut l1_adj = vec![vec![]; l1_nodes.len()];
    for (i, (ix, iy)) in l1_nodes.iter().enumerate() {
        for (j, (jx, jy)) in l1_nodes.iter().enumerate() {
            let distance = dist((*ix, *iy), (*jx, *jy));
            if distance <= 2000 {
                l1_adj[i].push((j, distance));
            }
        }
    }

    let start = Instant::now();
    let l1_paths = dijkstra_all::<usize, i32, _, _>(&0, |&n| l1_adj[n].iter().copied());
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);
}
