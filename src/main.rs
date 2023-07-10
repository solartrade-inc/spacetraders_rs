mod graph;
mod systems;

use graph::L2Graph;
use systems::System;

fn main() {
    let factions_json = std::fs::read_to_string("factions.json").unwrap();
    let factions: serde_json::Value = serde_json::from_str(&factions_json).unwrap();
    let charted_systems_json = std::fs::read_to_string("charted_systems.json").unwrap();
    let charted_systems: Vec<System> = serde_json::from_str(&charted_systems_json).unwrap();
    let graph = L2Graph::from_systems(&charted_systems);
    
    let SRC = "X1-ZU66-93668D";

    for f in factions.get("data").unwrap().as_array().unwrap() {
        let symbol = f.get("symbol").unwrap().as_str().unwrap();
        let hq = f.get("headquarters").unwrap().as_str().unwrap();
        println!("{}\t{}", symbol, hq);

        let x = graph.astar(SRC, hq).unwrap() / 60;
        println!("{}\t{}\t{} minutes", symbol, hq, x);
    }
}

