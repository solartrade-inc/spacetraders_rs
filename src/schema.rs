table! {
    agents (symbol) {
        symbol -> Varchar,
        bearer_token -> Text,
        agent -> Json,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    markets (symbol) {
        symbol -> Varchar,
        market -> Json,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    surveys (id) {
        id -> Int8,
        asteroid_symbol -> Text,
        survey -> Json,
        expires_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        extract_state -> Int4,
    }
}

allow_tables_to_appear_in_same_query!(agents, markets, surveys,);
