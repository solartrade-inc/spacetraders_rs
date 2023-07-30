use crate::models::*;

pub fn is_market(waypoint: &Waypoint) -> bool {
    waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE")
}
pub fn is_shipyard(waypoint: &Waypoint) -> bool {
    waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD")
}
pub fn is_asteroid(waypoint: &Waypoint) -> bool {
    waypoint._type == "ASTEROID_FIELD"
}

pub fn system_symbol(waypoint_symbol: &str) -> String {
    waypoint_symbol
        .split('-')
        .take(2)
        .collect::<Vec<&str>>()
        .join("-")
}

pub fn ship_symbol(callsign: &str, id: i32) -> String {
    // convert id to hex
    format!("{}-{:X}", callsign, id)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_system_symbol() {
        assert_eq!(system_symbol("X1-DK53-66197A"), "X1-DK53");
    }

    #[test]
    fn test_ship_symbol() {
        assert_eq!(ship_symbol("CALLSIGN_A", 1), "CALLSIGN_A-1");
        assert_eq!(ship_symbol("CALLSIGN_A", 10), "CALLSIGN_A-A");
        assert_eq!(ship_symbol("CALLSIGN_A", 255), "CALLSIGN_A-FF");
        assert_eq!(ship_symbol("CALLSIGN_A", 256), "CALLSIGN_A-100");
    }
}
