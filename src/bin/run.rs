use dotenvy::dotenv;
use log::*;
use spacetraders_rs::shipconfig::*;

use spacetraders_rs::{controller::Controller, scripts::mining::MiningController, util};

// todo: load 'shipconfig' from postgres on startup
// for now: define statically here
// the code isn't supposed to have gameplay 'data' hardcoded in it
const UNITED_ASTEROID_FIELD: &'static str = "X1-DK53-66197A";
const UNITED_SHIPYARD: &'static str = "X1-DK53-66197A";
lazy_static::lazy_static! {
    static ref CONFIG: AgentConfig = {
        let callsign: String = std::env::var("AGENT_CALLSIGN").expect("AGENT_CALLSIGN must be set");
        let mut config = AgentConfig {
            callsign: callsign,
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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init_timed();
    info!("Starting up...");

    // load agent (set bearer token)
    let mut controller = Controller::new(&CONFIG).load().await;

    // refetch ships: todo load from postgres instead
    controller.fetch_ships(1, 20).await;

    let mut executor_queue = vec![];
    for ship in &CONFIG.ships {
        if let ShipScript::Mining(mining_config) = &ship.script {
            let mining_controller = MiningController::new(&controller, &ship.symbol, &mining_config.asteroid_symbol);
            let executor = mining_controller.setup().await;
            executor_queue.push(executor);
        }
    }

    executor_queue.push(

    // call .step() on each executor in the queue, and push to the back of the queue
    loop {
        let front = executor_queue.pop();
        if let Some(mut executor) = front {
            let result = executor.step().await;
            executor_queue.push(executor);
        }
    }   
}

struct AutoBuy {

}

impl AutoBuy {
    async fn step() {

    }
}
