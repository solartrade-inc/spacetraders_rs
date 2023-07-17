use crate::models::*;

pub fn is_market(waypoint: &Waypoint) -> bool {
    waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE")
}
