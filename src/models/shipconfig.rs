use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub callsign: String,
    pub ships: Vec<ShipConfig>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShipConfig {
    pub symbol: String,
    pub module_config: Option<ModuleConfig>,
    pub script: ShipScript,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ShipScript {
    None,
    Mining(MiningConfig),
    // Trading,
    // Contract,
    // Exploring
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiningConfig {
    asteroid_symbol: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModuleConfig {}
