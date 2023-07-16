use dotenvy::dotenv;
use log::*;

use spacetraders_rs::api::Client;

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    info!("Starting up...");
    let client = Client::new();

    client.register("SOLARTRADE_INC", "UNITED").await;
}
