use dotenvy::dotenv;
use log::*;

use spacetraders_rs::{controller::Controller, mining::MiningController, util};

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init_timed();
    info!("Starting up...");

    // load agent (set bearer token)
    let callsign: String = std::env::var("AGENT_CALLSIGN").expect("AGENT_CALLSIGN must be set");
    let mut controller = Controller::new(&callsign).load().await;

    // refetch agent + ships
    let _ = controller.api_client.fetch_agent().await;
    let _ = controller.api_client.fetch_contracts(1, 20).await;
    let _ = controller.fetch_ships(1, 20).await;

    // control our command frigate
    let mut ship_controller = controller.ship_controller(3);
    ship_controller.flight_mode("CRUISE").await;

    let ship_system = ship_controller.ship().nav.system_symbol.clone();
    let waypoints = controller
        .api_client
        .fetch_system_waypoints(&ship_system)
        .await;
    debug!("Waypoints: {:?}", waypoints);

    let asteroid = waypoints.iter().find(|w| util::is_asteroid(w)).unwrap();

    // call into mining module
    let mining_controller = MiningController {
        par: controller,
        ship_idx: 3,
        asteroid_symbol: asteroid.symbol.clone(),
    };
    mining_controller.run().await;
}
