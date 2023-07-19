use dotenvy::dotenv;
use log::*;

use serde_json::Value;
use spacetraders_rs::api_client::ApiClient;
use spacetraders_rs::database::DatabaseClient;

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();

    let CALLSIGN: String = std::env::var("AGENT_CALLSIGN").expect("AGENT_CALLSIGN must be set");
    let FACTION: String = std::env::var("AGENT_FACTION").expect("AGENT_FACTION must be set");
    info!("Registering agent '{}' in '{}'...", CALLSIGN, FACTION);

    let api_client = ApiClient::new();
    let db_client = DatabaseClient::new();

    let resp = api_client.register(&CALLSIGN, &FACTION).await;
    assert!(
        resp.status.is_success(),
        "Failed to register agent: {} {}",
        resp.status,
        resp.body
    );
    let body: Value = serde_json::from_str(&resp.body).unwrap();
    let token = body["data"]["token"].as_str().unwrap();
    let agent = &body["data"]["agent"];

    db_client.save_agent(&CALLSIGN, token, agent).await;
}
