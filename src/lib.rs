#[macro_use]
extern crate diesel;

// interfaces
pub mod clients;
pub use clients::api_client;
pub use clients::database;

// models
pub mod models;
pub use models::db_models;
pub use models::schema;
pub use models::shipconfig;

pub mod controller;
pub mod scripts;

// tools
pub mod decision_tree;
pub mod util;
