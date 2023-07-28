use dotenvy::dotenv;
use log::*;
use spacetraders_rs::shipconfig::*;

use spacetraders_rs::{controller::Controller, scripts::mining::MiningController, util};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init_timed();
    info!("Starting up...");

    let callsign: String = std::env::var("AGENT_CALLSIGN").expect("AGENT_CALLSIGN must be set");

    // todo: load 'shipconfig' from postgres
    // for now: define inline here
    let config = AgentConfig {
        callsign: callsign,
        ships: vec![],
    };

    // load agent (set bearer token)
    let mut controller = Controller::new(&config.callsign).load().await;

    // refetch ships
    controller.fetch_ships(1, 20).await;

    let system_symbol = util::system_symbol(&controller.agent.headquarters);
    let waypoints = controller
        .api_client
        .fetch_system_waypoints(&system_symbol)
        .await;
    let asteroid = waypoints.iter().find(|w| util::is_asteroid(w)).unwrap();

    // better to have a queue of ships to run, which will scale better when we have more ships and are hitting rate limits
    // 3,4,5,6,7,8 all ore hounds
    let mut futs = vec![];
    for i in 3..=8 {
        let ship_symbol = format!("{}-{}", config.callsign, i);
        let mining_controller = MiningController::new(&controller, &ship_symbol, &asteroid.symbol);
        let fut = tokio::spawn(mining_controller.run());
        futs.push(fut);
    }
    futures::future::join_all(futs).await;
}
