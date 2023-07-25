use crate::models::*;

pub fn is_market(waypoint: &Waypoint) -> bool {
    waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE")
}

pub fn is_asteroid(waypoint: &Waypoint) -> bool {
    waypoint._type == "ASTEROID_FIELD"
}

pub fn system_symbol(waypoint_symbol: &str) -> String {
    waypoint_symbol
        .split("-")
        .take(2)
        .collect::<Vec<&str>>()
        .join("-")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_system_symbol() {
        assert_eq!(system_symbol("X1-DK53-66197A"), "X1-DK53");
    }
}
