use dotenvy::dotenv;
use log::*;
use spacetraders_rs::agentconfig::CONFIG;
use spacetraders_rs::runtime::Runtime;
use spacetraders_rs::shipconfig::*;
use spacetraders_rs::{controller::Controller, scripts::mining::MiningController};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init_timed();
    info!("Starting up...");

    // load agent (set bearer token)
    let mut controller = Controller::new(&CONFIG).load().await;

    // refetch ships: todo load from postgres instead
    controller.fetch_ships(1, 20).await;

    let mut runtime = Runtime::new();
    for ship in &CONFIG.ships {
        if let ShipScript::Mining(mining_config) = &ship.script {
            let mining_controller =
                MiningController::new(&controller, &ship.symbol, &mining_config.asteroid_symbol);
            let executor = mining_controller.setup().await;
            runtime.add(Box::new(executor), 50).await;
        }
    }

    runtime.run().await;
}
