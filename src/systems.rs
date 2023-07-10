use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Trait {
    pub name: String,
    pub description: String,
    pub symbol: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Waypoint {
    pub symbol: String,
    pub x: i32,
    pub y: i32,
    #[serde(rename = "type")]
    pub waypoint_type: String,
    pub traits: Option<Vec<Trait>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct System {
    pub symbol: String,
    #[serde(rename = "type")]
    pub system_type: String,
    pub x: i32,
    pub y: i32,
    pub waypoints: Vec<Waypoint>,
}
