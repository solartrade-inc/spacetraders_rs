use dotenvy::dotenv;
use log::*;

use spacetraders_rs::api::Client as Controller;

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    info!("Starting up...");
    let client = Controller::new();

    // load agent (set bearer token)
    let agent = client.load_callsign("SOLARTRADE_INC").await;

    // refetch agent
    client.fetch_agent().await;
    // refetch contracts
    client.fetch_contracts().await;
    // refetch ships
    client.fetch_ships().await;

    // grab our command frigate, and send it to all the marketplaces in the starting system
    let ship = client.ships.get(0).controller(); // (clones internals to some degree)
    ship.flight_mode('CRUISE').await;

    client.fetch_system(ship.location).await;
    let system = client.systems.get(ship.location).unwrap();

    for waypoint in system.waypoints.iter() {
        if waypoint.is_market() {
            debug!("Navigating to {}", waypoint.symbol);
            ship.navigate_to(waypoint.symbol).await;
            ship.fetch_market().await;
            ship.refuel().await;
        }
    }

    // We should now have full info of all markets in the starting system
    // Together we the asteroid fields, we can now start to evalulate our
    // mining and trade routes.
}
