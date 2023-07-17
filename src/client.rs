use crate::db_models::Agent;
use crate::diesel::ExpressionMethods;
use crate::diesel::OptionalExtension as _;
use crate::schema::*;
use diesel::QueryDsl as _;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl as _;
use hyper::Uri;
use log::*;
use serde_json::json;
use std::env;

pub struct Client {
    pub db: Pool<AsyncPgConnection>,
    pub inner: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
}

impl Client {
    pub fn new() -> Self {
        let https = hyper_tls::HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        let db_pool = {
            let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            let manager = AsyncDieselConnectionManager::new(&database_url);
            Pool::builder(manager).max_size(2).build().unwrap()
        };
        Self {
            inner: client,
            db: db_pool,
        }
    }

    /// Register a new agent with the SpaceTraders API, and store the token in the database.
    pub async fn register(&self, callsign: &str, faction: &str) {
        let uri: Uri = "https://api.spacetraders.io/v2/register".parse().unwrap();
        let payload = json! ({
            "faction": faction,
            "symbol": callsign,
        });
        let req = hyper::Request::post(uri)
            .header("Content-Type", "application/json")
            .body(hyper::Body::from(payload.to_string()))
            .unwrap();
        let res = self.inner.request(req).await.unwrap();
        let status = res.status();
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = std::str::from_utf8(&body_bytes).unwrap();
        info!("Body: {:?}", body);

        if !status.is_success() {
            panic!("Failed to register: {}", status);
        }

        let json: serde_json::Value = serde_json::from_str(body).unwrap();
        let token = json["data"]["token"].as_str().unwrap();
        let agent = &json["data"]["agent"];
        info!("Token: {}", token);

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
}
