use dotenvy::dotenv;
use log::*;
use spacetraders_rs::{agentconfig::CONFIG, controller::Controller, util};

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();

    let mut controller = Controller::new(&CONFIG).load().await;

    if controller.agent.lock().unwrap().is_none() {
        info!("No agent found. Registering...");
        controller.register().await;
    } else {
        info!("Agent found. Continuing...");
    }

    controller.fetch_agent().await;
    controller.fetch_ships(1, 20).await;
    info!("Number of ships: {}", controller.ships.len());

    controller.fetch_contracts(1, 20).await;
    let contracts = controller.contracts.lock().unwrap();
    info!("Number of contracts: {}", contracts.len());

    if !contracts[0].accepted {
        controller.accept_contract(&contracts[0].id).await;
    }
    let agent_guard = controller.agent.lock().unwrap();
    let agent = agent_guard.as_ref().unwrap();
    info!("Agent: {} ${}", agent.symbol, agent.credits);

    // buy ore hound
    if controller.ships.len() == 2 {
        let ship_system = util::system_symbol(&agent.headquarters);
        // @@ should read systems and waypoints from memory, not from api
        let waypoints = controller
            .api_client
            .fetch_system_waypoints(&ship_system)
            .await;
        let shipyard = waypoints.iter().find(|w| util::is_shipyard(w)).unwrap();
        // @@ this is bugged because agent_guard is still locked
        controller
            .buy_ship("SHIP_ORE_HOUND", &shipyard.symbol)
            .await;
    }
}
