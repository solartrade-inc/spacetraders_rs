use crate::db_models::Agent;
use crate::diesel::ExpressionMethods;
use crate::diesel::OptionalExtension as _;
use crate::models::Market;
use crate::schema::*;
use diesel::QueryDsl as _;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl as _;

use serde_json::Value;
use std::env;

pub struct DatabaseClient {
    pub db: Pool<AsyncPgConnection>,
}

impl DatabaseClient {
    pub fn new() -> Self {
        let db_pool = {
            let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            let manager = AsyncDieselConnectionManager::new(database_url);
            Pool::builder(manager).max_size(2).build().unwrap()
        };
        Self { db: db_pool }
    }

    pub async fn load_agent(&self, callsign: &str) -> Agent {
        let mut conn = self.db.get().await.unwrap();
        let agent: Option<Agent> = agents::table
            .select((
                agents::symbol,
                agents::bearer_token,
                agents::agent,
                agents::created_at,
                agents::updated_at,
            ))
            .filter(agents::symbol.eq(callsign))
            .first(&mut conn)
            .await
            .optional()
            .unwrap();
        agent.unwrap()
    }

    pub async fn save_agent(&self, callsign: &str, token: &str, agent: &Value) {
        let mut conn = self.db.get().await.unwrap();
        diesel::insert_into(agents::table)
            .values((
                agents::symbol.eq(callsign),
                agents::bearer_token.eq(token),
                agents::agent.eq(agent),
                agents::created_at.eq(diesel::dsl::now),
                agents::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    pub async fn upsert_market(&self, market: &Market) {
        let mut conn = self.db.get().await.unwrap();
        let market_val: Value = serde_json::to_value(market).unwrap();
        diesel::insert_into(markets::table)
            .values((
                markets::symbol.eq(&market.symbol),
                markets::market.eq(&market_val),
                markets::createdAt.eq(diesel::dsl::now),
                markets::updatedAt.eq(diesel::dsl::now),
            ))
            .on_conflict(markets::symbol)
            .do_update()
            .set((
                markets::market.eq(&market_val),
                markets::updatedAt.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
    }
}