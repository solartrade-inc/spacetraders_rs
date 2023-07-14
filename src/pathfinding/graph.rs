use pathfinding::prelude::*;
use std::cmp::max;
use std::time::Instant;
use std::vec::Vec;

use crate::systems::System;

const START_FUEL: i32 = 1500;
const MAX_FUEL: i32 = 1500;
const SHIP_SPEED: i32 = 30;
const MAX_JUMPGATE: i32 = 2000;
const MAX_WARP: i32 = 10000;
const FUEL_SEGMENT: i32 = 150;
const DRIFT_FUEL_COST: i32 = 1;

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


fn dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    ((((a.0 - b.0) as i64).pow(2) + ((a.1 - b.1) as i64).pow(2)) as f64)
        .sqrt()
        .round() as i32
}

pub struct L2Graph {
    l2_nodes: Vec<Node>,
    l2_nodes_name: Vec<String>,
    l2_adj: Vec<Vec<(usize, Edge, i32, i32, bool)>>,
}

impl L2Graph {
    pub fn from_systems(charted_systems: &Vec<System>) -> Self {
        let (nodes, nodes_name, adj) = build_l2(charted_systems);
        L2Graph {
            l2_nodes: nodes,
            l2_nodes_name: nodes_name,
            l2_adj: adj,
        }
    }
    
    pub fn dijkstra(&self, src: &str) {
        let src = self.l2_nodes_name.iter().position(|x| x == src).unwrap();
        let start = Instant::now();
        let result = dijkstra_all::<(usize, i32), i32, _, _>(
            &(src, MAX_FUEL),
            |&n| {
                let node_idx = n.0;
                let node_fuel = n.1;
                self.l2_adj[node_idx]
                    .iter()
                    .filter_map(move |&(e, _, w, f, dest_is_refuel)| {
                        if n.1 - f >= 0 {
                            if dest_is_refuel {
                                let f1 = MAX_FUEL;
                                Some(((e, f1), w))
                            } else {
                                let f1 = (node_fuel - f) / FUEL_SEGMENT * FUEL_SEGMENT;
                                Some(((e, f1), w))
                            }
                        } else {
                            None
                        }
                    })
            },
        );
        let duration = start.elapsed();
        println!("Time elapsed: {:?}", duration);
    }

    pub fn astar(&self, src: &str, dest: &str) -> Option<i32> {    
        let src = self.l2_nodes_name.iter().position(|x| x == src).unwrap();
        let dest = self.l2_nodes_name.iter().position(|x| x == dest).unwrap();
    
        let dist_to_dest = self.l2_nodes
            .iter()
            .map(|x| {
                let sys_i = match x {
                    JumpgateWaypoint(SystemPoint(x, y), _) => SystemPoint(*x, *y),
                    MarketWaypoint(SystemPoint(x, y), _) => SystemPoint(*x, *y),
                    System(SystemPoint(x, y)) => SystemPoint(*x, *y),
                };
                let sys_j = match &self.l2_nodes[dest] {
                    JumpgateWaypoint(SystemPoint(x, y), _) => SystemPoint(*x, *y),
                    MarketWaypoint(SystemPoint(x, y), _) => SystemPoint(*x, *y),
                    System(SystemPoint(x, y)) => SystemPoint(*x, *y),
                };
                dist((sys_i.0, sys_i.1), (sys_j.0, sys_j.1))
            })
            .collect::<Vec<_>>();
    
        let start = Instant::now();
        let result = astar::<(usize, i32), i32, _, _, _, _>(
            &(src, START_FUEL),
            |&n| {
                let node_idx = n.0;
                let node_fuel = n.1;
                self.l2_adj[node_idx]
                    .iter()
                    .filter_map(move |&(e, _, w, f, dest_is_refuel)| {
                        if n.1 - f >= 0 {
                            if dest_is_refuel {
                                let f1 = MAX_FUEL;
                                Some(((e, f1), w))
                            } else {
                                let f1 = (node_fuel - f) / FUEL_SEGMENT * FUEL_SEGMENT;
                                Some(((e, f1), w))
                            }
                        } else {
                            None
                        }
                    })
            },
            |&n| dist_to_dest[n.0] / 10,
            |&n| n.0 == dest,
        );
        let duration = start.elapsed();
        println!("Time elapsed: {:?}", duration);
    
        let (path, duration) = result.unwrap();
        let names: Vec<&str> = path.iter().map(|&i| self.l2_nodes_name[i.0].as_str()).collect();
    
        println!(
            "Planned route from {} to {} under fuel constraint: {MAX_FUEL}",
            self.l2_nodes_name[src], self.l2_nodes_name[dest]
        );
        let mut duration_acc = 0;
        let mut fuel_acc = START_FUEL;
        for i in 0..path.len() - 1 {
            let edge = self.l2_adj[path[i].0]
                .iter()
                .filter(|&&(j, _, _w, f, dest_is_refuel)| {
                    if j != path[i + 1].0 {
                        return false;
                    }
                    if path[i].1 - f >= 0 {
                        if dest_is_refuel {
                            let f1 = MAX_FUEL;
                            f1 == path[i + 1].1
                        } else {
                            let f1 = (path[i].1 - f) / FUEL_SEGMENT * FUEL_SEGMENT;
                            f1 == path[i + 1].1
                        }
                    } else {
                        false
                    }
                })
                .min_by(|&&(_, _, w1, _, _), &&(_, _, w2, _, _)| w1.cmp(&w2))
                .unwrap();
            duration_acc += edge.2;
            fuel_acc = (fuel_acc - edge.3) / FUEL_SEGMENT * FUEL_SEGMENT;
            println!(
                "{:?}\t{:14}  ->  {:14}\t{}s\t{}s\t{}\t{}",
                edge.1,
                names[i],
                names[i + 1],
                edge.2,
                duration_acc,
                edge.3,
                fuel_acc,
            );
            if edge.4 {
                fuel_acc = MAX_FUEL;
                println!("Refuel\t\t{:14}\t{:14}\t\t\t\t\t{}", names[i+1], "", fuel_acc);
            }
        }
    
        println!("Total duration: {}s", duration);
        Some(duration)
    }
}

fn build_l2(charted_systems: &Vec<System>) -> (Vec<Node>, Vec<String>, Vec<Vec<(usize, Edge, i32, i32, bool)>>) {
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
            } else if is_market
                // || waypoint.symbol == "X1-FT59-41745B" || waypoint.symbol == "X1-QP42-01002A"
            {
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
            if i == j {
                continue;
            }
            let dest_is_refuel: bool = match &node_j {
                JumpgateWaypoint(_, _) => false,
                MarketWaypoint(_, _) => true,
                System(_) => false,
            };

            if let (JumpgateWaypoint(j1, _), JumpgateWaypoint(j2, _)) = &(node_i, node_j) {
                let distance = dist((j1.0, j1.1), (j2.0, j2.1));
                if distance <= MAX_JUMPGATE {
                    let duration = max(60, ((distance as f64) / 10f64).round() as i32);
                    l2_adj[i].push((j, Edge::Jumpgate, duration, 0, dest_is_refuel));
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
                        l2_adj[i].push((j, Edge::Nav(CRUISE), duration, fuel, dest_is_refuel));
                    }
                }
                // nav BURN
                {
                    let fuel = nav_dist * 2;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 2. / 15.;
                    let duration = 15 + (nav_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL {
                        l2_adj[i].push((j, Edge::Nav(BURN), duration, fuel, dest_is_refuel));
                    }
                }
                // nav DRIFT
                {
                    let fuel = DRIFT_FUEL_COST;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 0.1 / 15.;
                    let duration = 15 + (nav_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL {
                        l2_adj[i].push((j, Edge::Nav(DRIFT), duration, fuel, dest_is_refuel));
                    }
                }
            } else {
                // warp: BURN
                {
                    let fuel = sys_dist * 2;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 2. / 20.;
                    let duration = 15 + (sys_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL {
                        l2_adj[i].push((j, Edge::Warp(BURN), duration, fuel, dest_is_refuel));
                    }
                }
                // warp: CRUISE
                {
                    let fuel = sys_dist;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 1. / 20.;
                    let duration = 15 + (sys_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL {
                        l2_adj[i].push((j, Edge::Warp(CRUISE), duration, fuel, dest_is_refuel));
                    }
                }
                // warp: DRIFT
                {
                    let fuel = DRIFT_FUEL_COST;
                    let effective_speed: f64 = SHIP_SPEED as f64 * 0.1 / 20.;
                    let duration = 15 + (sys_dist as f64 / effective_speed).round() as i32;
                    if fuel <= MAX_FUEL && sys_dist <= MAX_WARP {
                        l2_adj[i].push((j, Edge::Warp(DRIFT), duration, fuel, dest_is_refuel));
                    }
                }
            }
        }
    }

    let num_edges = l2_adj.iter().map(|x| x.len()).sum::<usize>();
    println!("num edges: {}", num_edges);

    (l2_nodes, l2_nodes_name, l2_adj)
}
