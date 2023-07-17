use diesel_async::RunQueryDsl as _;
use dotenvy::dotenv;
use log::*;

use spacetraders_rs::client::Client;
use spacetraders_rs::schema::*;

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    info!("Starting up...");
    let client = Client::new();

    info!("Deleting all agents...");
    let mut conn = client.db.get().await.unwrap();
    diesel::delete(agents::table)
        .execute(&mut conn)
        .await
        .unwrap();
    info!("Deleted all agents");
}
