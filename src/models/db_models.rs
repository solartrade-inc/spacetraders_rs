use crate::schema::*;
use diesel::Queryable;
use diesel::QueryableByName;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, QueryableByName, Queryable, Debug, Clone)]
#[diesel(table_name = agents)]
pub struct Agent {
    pub symbol: String,
    pub bearer_token: String,
    pub agent: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, QueryableByName, Queryable, Debug, Clone)]
#[diesel(table_name = ships)]
pub struct Ship {}
