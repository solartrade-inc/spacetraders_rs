use diesel_async::RunQueryDsl as _;
use dotenvy::dotenv;
use log::*;

use spacetraders_rs::database::DatabaseClient;
use spacetraders_rs::schema::*;

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    info!("Starting up...");
    let db_client = DatabaseClient::new();
    let mut conn = db_client.db.get().await.unwrap();

    info!("Deleting all agents...");
    diesel::delete(agents::table)
        .execute(&mut conn)
        .await
        .unwrap();
    info!("Deleted all agents");

    info!("Deleting all markets...");
    diesel::delete(markets::table)
        .execute(&mut conn)
        .await
        .unwrap();
    info!("Deleting all ships...");

    info!("Deleting all surveys...");
    diesel::delete(surveys::table)
        .execute(&mut conn)
        .await
        .unwrap();
    info!("Deleted all surveys");
}
