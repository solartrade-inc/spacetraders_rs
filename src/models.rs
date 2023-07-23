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
    // pub reactor, engine, modules
    pub mounts: Vec<ShipMount>,
    pub cargo: ShipCargo,
    pub fuel: ShipFuel,

    // !! not in API response (yet)
    pub cooldown: Option<ShipCooldown>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ShipMount {
    pub symbol: String,
    // name ,descr,
    pub strength: Option<u32>,
    pub deposits: Option<Vec<String>>,
    pub requirements: ShipMountRequirements,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ShipMountRequirements {
    pub power: i32,
    pub crew: i32,
    pub slots: Option<i32>,
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

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Survey {
    pub signature: String,
    pub symbol: String,
    pub deposits: Vec<Symbol>,
    pub expiration: DateTime<Utc>,
    pub size: String,
}

#[derive(Debug, Clone)]
pub struct WrappedSurvey {
    pub id: i64,
    pub survey: Survey,
}
impl WrappedSurvey {
    pub fn inner(&self) -> &Survey {
        &self.survey
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipCooldown {
    pub ship_symbol: String,
    pub total_seconds: u32,
    pub remaining_seconds: u32,
    pub expiration: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipExtraction {
    pub ship_symbol: String,
    #[serde(rename = "yield")]
    pub _yield: ShipExtractionYield,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ShipExtractionYield {
    pub symbol: String,
    pub units: u32,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketTransaction {
    pub waypoint_symbol: String,
    pub ship_symbol: String,
    pub trade_symbol: String,
    #[serde(rename = "type")]
    pub _type: String,
    pub units: u32,
    pub price_per_unit: u32,
    pub total_price: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
    pub code: u16,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_survey_deserialize() {
        let data = r#"{"data":{"cooldown":{"shipSymbol":"SOLARTRADE_INC-3","totalSeconds":70,"remainingSeconds":69,"expiration":"2023-07-22T12:31:37.322Z"},"surveys":[{"signature":"X1-JK96-45265A-FE9FBF","symbol":"X1-JK96-45265A","deposits":[{"symbol":"AMMONIA_ICE"},{"symbol":"AMMONIA_ICE"},{"symbol":"AMMONIA_ICE"},{"symbol":"ALUMINUM_ORE"},{"symbol":"SILICON_CRYSTALS"},{"symbol":"AMMONIA_ICE"}],"expiration":"2023-07-22T12:41:45.322Z","size":"SMALL"}]}}"#;
        let mut body: serde_json::Value = serde_json::from_str(data).unwrap();
        let surveys: Vec<Survey> = serde_json::from_value(body["data"]["surveys"].take()).unwrap();
        assert_eq!(surveys.len(), 1);
        assert_eq!(surveys[0].deposits.len(), 6);

        let serialized: String = serde_json::to_string(&surveys[0]).unwrap();
        assert_eq!(
            serialized,
            r#"{"signature":"X1-JK96-45265A-FE9FBF","symbol":"X1-JK96-45265A","deposits":[{"symbol":"AMMONIA_ICE"},{"symbol":"AMMONIA_ICE"},{"symbol":"AMMONIA_ICE"},{"symbol":"ALUMINUM_ORE"},{"symbol":"SILICON_CRYSTALS"},{"symbol":"AMMONIA_ICE"}],"expiration":"2023-07-22T12:41:45.322Z","size":"SMALL"}"#
        );
    }
}
