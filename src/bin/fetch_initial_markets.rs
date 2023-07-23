use dotenvy::dotenv;
use log::*;

use spacetraders_rs::{controller::Controller, util};

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init_timed();
    info!("Starting up...");

    // load agent (set bearer token)
    let callsign: String = std::env::var("AGENT_CALLSIGN").expect("AGENT_CALLSIGN must be set");
    let mut controller = Controller::new(&callsign).load().await;

    // refetch agent
    let _ = controller.api_client.fetch_agent().await;
    // refetch contracts
    let _ = controller.api_client.fetch_contracts(1, 20).await;
    // refetch ships
    controller.fetch_ships(1, 20).await;

    // grab our command frigate, and send it to all the marketplaces in the starting system
    let mut ship_controller = controller.ship_controller(1);
    ship_controller.flight_mode("CRUISE").await;

    let ship_system = ship_controller.ship().nav.system_symbol.clone();
    let waypoints = controller
        .api_client
        .fetch_system_waypoints(&ship_system)
        .await;
    // let system = client.systems.get(ship.location).unwrap();

    debug!("Waypoints: {:?}", waypoints);

    for waypoint in waypoints.iter() {
        if util::is_market(waypoint) {
            debug!("Navigating to {}", waypoint.symbol);
            let mut ship_controller = controller.ship_controller(1);
            ship_controller.navigate(&waypoint.symbol).await;
            ship_controller.sleep_for_navigation().await;
            ship_controller.fetch_market().await;
            ship_controller.refuel().await;

            let market = controller.markets.get(&waypoint.symbol).unwrap();
            debug!("Market: {:?}", market);
        }
    }
    // We should now have full info of all markets in the starting system
    // Together we the asteroid fields, we can now start to evalulate our
    // mining and trade routes.
}
