use crate::shipconfig::*;

// todo: load 'shipconfig' from postgres on startup
// for now: define statically here
// the code isn't supposed to have gameplay 'data' hardcoded in it, so lets keep it contrained to this file

const UNITED_ASTEROID_FIELD: &str = "X1-DK53-66197A";
const UNITED_SHIPYARD: &str = "X1-DK53-66197A";
lazy_static::lazy_static! {
    pub static ref CONFIG: AgentConfig = {
        let callsign: String = std::env::var("AGENT_CALLSIGN").expect("AGENT_CALLSIGN must be set");
        let mut config = AgentConfig {
            callsign,
            ships: vec![],
        };
        // 20 ships
        for i in 3..23 {
            let ship_symbol = format!("{}-{}", config.callsign, i); // @@ hex
            let ship_config = ShipConfig {
                symbol: ship_symbol.clone(),
                shipyard: UNITED_SHIPYARD.into(),
                module_config: Some(ModulesConfig {
                    install_location: Some(UNITED_SHIPYARD.into()),
                    modules: vec![
                        ModuleConfig {
                            module: "MOUNT_SURVEYOR_I".into(),
                            source: None,
                        },
                        ModuleConfig {
                            module: "MOUNT_MINING_LASER_II".into(),
                            source: Some(UNITED_SHIPYARD.into()),
                        },
                        ModuleConfig {
                            module: "MOUNT_MINING_LASER_II".into(),
                            source: Some(UNITED_SHIPYARD.into()),
                        },
                    ],
                }),
                script: ShipScript::Mining(MiningConfig {
                    asteroid_symbol: UNITED_ASTEROID_FIELD.into(),
                }),
            };
            config.ships.push(ship_config);
        }
        config
    };
}
