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
    extractions (id) {
        id -> Int4,
        extractor -> Varchar,
        extractor_mounts -> Varchar,
        symbol -> Varchar,
        units -> Varchar,
        surveyId -> Nullable<Int8>,
        asteroidField -> Varchar,
        asteroidFieldTraits -> Varchar,
        createdAt -> Timestamptz,
    }
}

table! {
    factions (symbol) {
        symbol -> Varchar,
        faction -> Json,
        createdAt -> Timestamptz,
        updatedAt -> Timestamptz,
    }
}

table! {
    knex_migrations (id) {
        id -> Int4,
        name -> Nullable<Varchar>,
        batch -> Nullable<Int4>,
        migration_time -> Nullable<Timestamptz>,
    }
}

table! {
    knex_migrations_lock (index) {
        index -> Int4,
        is_locked -> Nullable<Int4>,
    }
}

table! {
    market_transactions (id) {
        id -> Int4,
        waypointSymbol -> Varchar,
        shipSymbol -> Varchar,
        tradeSymbol -> Varchar,
        #[sql_name = "type"]
        type_ -> Varchar,
        units -> Int4,
        pricePerUnit -> Int4,
        totalPrice -> Int4,
        timestamp -> Timestamptz,
        createdAt -> Timestamptz,
    }
}

table! {
    markets (symbol) {
        symbol -> Varchar,
        market -> Json,
        createdAt -> Timestamptz,
        updatedAt -> Timestamptz,
    }
}

table! {
    probe_system_reservation (shipSymbol) {
        shipSymbol -> Varchar,
        systemSymbol -> Varchar,
        createdAt -> Timestamptz,
        updatedAt -> Timestamptz,
    }
}

table! {
    ships (symbol) {
        symbol -> Varchar,
        config -> Json,
        ship -> Nullable<Json>,
        cooldown -> Nullable<Json>,
        isPurchased -> Bool,
        createdAt -> Timestamptz,
        updatedAt -> Timestamptz,
    }
}

table! {
    shipyards (symbol) {
        symbol -> Varchar,
        shipyard -> Json,
        createdAt -> Timestamptz,
        updatedAt -> Timestamptz,
    }
}

table! {
    surveys (id) {
        id -> Int4,
        signature -> Varchar,
        survey -> Json,
        extractValue -> Float4,
        isExhausted -> Bool,
        isExpired -> Bool,
        size -> Varchar,
        expiresAt -> Timestamptz,
        asteroidField -> Varchar,
        asteroidField_traits -> Varchar,
        surveyor -> Varchar,
        surveyorMounts -> Varchar,
        createdAt -> Timestamptz,
        updatedAt -> Timestamptz,
    }
}

table! {
    systems (symbol) {
        symbol -> Varchar,
        #[sql_name = "type"]
        type_ -> Varchar,
        x -> Int4,
        y -> Int4,
        waypoints -> Json,
        hasJumpgate -> Bool,
        hasUncharted -> Bool,
        createdAt -> Timestamptz,
        updatedAt -> Timestamptz,
    }
}

allow_tables_to_appear_in_same_query!(
    agents,
    extractions,
    factions,
    knex_migrations,
    knex_migrations_lock,
    market_transactions,
    markets,
    probe_system_reservation,
    ships,
    shipyards,
    surveys,
    systems,
);
