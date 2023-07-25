use dotenvy::dotenv;
use log::*;

use spacetraders_rs::{controller::Controller, mining::{MiningController, self}, util};

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
    let mut futs = vec![];
    for i in 3..=8 {
        let ship_symbol = format!("{}-{}", callsign, i);
        let mining_controller = MiningController::new(&controller, &ship_symbol, &asteroid.symbol);
        let fut = tokio::spawn(mining_controller.run());
        futs.push(fut);
    }
    futures::future::join_all(futs).await;
}
