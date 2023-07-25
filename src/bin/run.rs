use dotenvy::dotenv;
use log::*;

use spacetraders_rs::{controller::Controller, mining::MiningController, util};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init_timed();
    info!("Starting up...");

    // load agent (set bearer token)
    let callsign: String = std::env::var("AGENT_CALLSIGN").expect("AGENT_CALLSIGN must be set");
    let mut controller = Controller::new(&callsign).load().await;

    // refetch ships
    controller.fetch_ships(1, 20).await;

    let system_symbol = util::system_symbol(&controller.agent.headquarters);
    let waypoints = controller
        .api_client
        .fetch_system_waypoints(&system_symbol)
        .await;
    let asteroid = waypoints.iter().find(|w| util::is_asteroid(w)).unwrap();

    // 3,4,5,6,7,8 all ore hounds

    // call into mining module
    let mining_controller = MiningController {
        par: controller.clone(),
        ship_idx: 3,
        asteroid_symbol: asteroid.symbol.clone(),
    };
    let mining_controller_2 = MiningController {
        par: controller.clone(),
        ship_idx: 4,
        asteroid_symbol: asteroid.symbol.clone(),
    };
    let fut = vec![
        tokio::spawn(mining_controller.run()),
        tokio::spawn(mining_controller_2.run()),
    ];
    futures::future::join_all(fut).await;
}
