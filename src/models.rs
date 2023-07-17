use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct List<T> {
    pub data: Vec<T>,
    pub meta: Meta,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Data<T> {
    pub data: T,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Symbol {
    pub symbol: String,
    // name, descr
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Meta {
    pub total: u32,
    pub page: u32,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Ship {
    pub symbol: String,
    pub registration: ShipRegistration,
    pub nav: ShipNav,
    // pub crew
    // pub frame
    // pub reactor, engine, modules, mounts
    pub cargo: ShipCargo,
    pub fuel: ShipFuel,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ShipRegistration {
    pub name: String,
    #[serde(rename = "factionSymbol")]
    pub faction_symbol: String,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ShipFuel {
    pub current: u32,
    pub capacity: u32,
    // pub consumed:
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ShipCargo {
    pub capacity: u32,
    pub units: u32,
    pub inventory: Vec<ShipCargoGood>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ShipCargoGood {
    pub symbol: String,
    pub units: u32,
    // pub name, description
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ShipNav {
    #[serde(rename = "systemSymbol")]
    pub system_symbol: String,
    #[serde(rename = "waypointSymbol")]
    pub waypoint_symbol: String,
    pub route: ShipNavRoute,
    pub status: String,
    #[serde(rename = "flightMode")]
    pub flight_mode: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ShipNavRoute {
    // destination, departure
    #[serde(rename = "departureTime")]
    pub departure_time: DateTime<Utc>,
    pub arrival: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Waypoint {
    pub symbol: String,
    #[serde(rename = "type")]
    pub _type: String,
    #[serde(rename = "systemSymbol")]
    pub system_symbol: String,
    pub x: i32,
    pub y: i32,
    // orbitals, faction,
    pub traits: Vec<Symbol>,
    // chart
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Market {
    pub symbol: String,
    pub exports: Vec<Symbol>,
    pub imports: Vec<Symbol>,
    pub exchange: Vec<Symbol>,
    // transactions
    #[serde(rename = "tradeGoods")]
    pub trade_goods: Vec<MarketTradeGood>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct MarketTradeGood {
    pub symbol: String,
    #[serde(rename = "tradeVolume")]
    pub trade_volume: u32,
    #[serde(rename = "purchasePrice")]
    pub purchase_price: u32,
    #[serde(rename = "sellPrice")]
    pub sell_price: u32,
    pub supply: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Agent {
    #[serde(rename = "accountId")]
    pub account_id: String,
    pub symbol: String,
    pub headquarters: String,
    pub credits: i64,
    #[serde(rename = "startingFaction")]
    pub starting_faction: String,
}
