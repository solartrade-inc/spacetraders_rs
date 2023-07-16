



fn simulate(asteroidField: &X, markets: &Vec<Y>) {
    
    // gain resource X
    // sell resource X to location Y0
    // sell resource X to location Y1

    let rv_deposit = map! {
        "iron_ore" => 1.0,
        "copper_ore" => 1.0,
        "silicon_ore" => 1.0,
        "water" => 2.0,
    };

    let root = Node::new("root", 0.0);
    for (resource, deposit) in rv_deposit.iter() {
        for market in markets.iter() {

        }
    }

}
