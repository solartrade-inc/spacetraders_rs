use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub callsign: String,
    pub faction: String,
    pub email: Option<String>,
    pub ships: Vec<ShipConfig>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShipConfig {
    pub symbol: String,
    pub shipyard: String,
    pub module_config: Option<ModulesConfig>,
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
    pub asteroid_symbol: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModulesConfig {
    pub install_location: Option<String>,
    pub modules: Vec<ModuleConfig>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModuleConfig {
    pub module: String,
    pub source: Option<String>,
}
