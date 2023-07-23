use crate::db_models::Agent;
use crate::diesel::ExpressionMethods;
use crate::diesel::OptionalExtension as _;
use crate::models::Market;
use crate::models::Survey;

use crate::models::WrappedSurvey;
use crate::schema::*;
use diesel::QueryDsl as _;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl as _;

use serde::Deserialize;
use serde::Serialize;
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
                markets::created_at.eq(diesel::dsl::now),
                markets::updated_at.eq(diesel::dsl::now),
            ))
            .on_conflict(markets::symbol)
            .do_update()
            .set((
                markets::market.eq(&market_val),
                markets::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    pub async fn load_market(&self, symbol: &str) -> Market {
        #[derive(Serialize, Deserialize, QueryableByName, Queryable, Debug, Clone)]
        #[diesel(table_name = markets)]
        struct ResultRow {
            symbol: String,
            market: Value,
            created_at: chrono::NaiveDateTime,
            updated_at: chrono::NaiveDateTime,
        }

        let mut conn = self.db.get().await.unwrap();
        let row: Option<ResultRow> = markets::table
            .select((
                markets::symbol,
                markets::market,
                markets::created_at,
                markets::updated_at,
            ))
            .filter(markets::symbol.eq(symbol))
            .first(&mut conn)
            .await
            .optional()
            .unwrap();
        serde_json::from_value(row.unwrap().market).unwrap()
    }

    pub async fn insert_surveys(&self, surveys: &Vec<Survey>) -> Vec<WrappedSurvey> {
        let mut conn = self.db.get().await.unwrap();
        let inserts = surveys
            .iter()
            .map(|s: &Survey| {
                let val: Value = serde_json::to_value(s).unwrap();
                (
                    surveys::asteroid_symbol.eq(&s.symbol),
                    surveys::survey.eq(val),
                    surveys::expires_at.eq(&s.expiration),
                    surveys::created_at.eq(diesel::dsl::now),
                    surveys::updated_at.eq(diesel::dsl::now),
                    surveys::extract_state.eq(0),
                )
            })
            .collect::<Vec<_>>();
        let rows: Vec<(i64, Value)> = diesel::insert_into(surveys::table)
            .values(inserts)
            .returning((surveys::id, surveys::survey))
            .load(&mut conn)
            .await
            .unwrap();
        rows.into_iter()
            .map(|r| WrappedSurvey {
                id: r.0,
                survey: serde_json::from_value(r.1).unwrap(),
            })
            .collect()
    }

    pub async fn load_surveys(&self, state: i32) -> Vec<WrappedSurvey> {
        let mut conn = self.db.get().await.unwrap();
        let rows: Vec<(
            i64,
            String,
            Value,
            chrono::NaiveDateTime,
            chrono::NaiveDateTime,
        )> = surveys::table
            .select((
                surveys::id,
                surveys::asteroid_symbol,
                surveys::survey,
                surveys::expires_at,
                surveys::updated_at,
            ))
            .filter(surveys::extract_state.eq(&state))
            .filter(surveys::expires_at.gt(diesel::dsl::now))
            .load(&mut conn)
            .await
            .unwrap();
        rows.into_iter()
            .map(|r| WrappedSurvey {
                id: r.0,
                survey: serde_json::from_value(r.2).unwrap(),
            })
            .collect()
    }

    pub async fn update_survey_state(&self, survey: &WrappedSurvey, state: i32) {
        let mut conn = self.db.get().await.unwrap();
        diesel::update(surveys::table)
            .filter(surveys::id.eq(survey.id))
            .set((
                surveys::updated_at.eq(diesel::dsl::now),
                surveys::extract_state.eq(state),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
    }
}
