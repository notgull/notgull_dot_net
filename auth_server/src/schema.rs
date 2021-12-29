table! {
    ipaddresses (id) {
        id -> Int4,
        user_id -> Int4,
        ip_address -> Varchar,
        last_used -> Timestamp,
    }
}

table! {
    managedusers (id) {
        id -> Int4,
        salt -> Bytea,
        email -> Varchar,
        username -> Varchar,
        login_attempts -> Int4,
        blocked_on -> Nullable<Timestamp>,
        shadow -> Int4,
    }
}

table! {
    shadow (id) {
        id -> Int4,
        hashed_password -> Bytea,
    }
}

table! {
    users (id) {
        id -> Int4,
        uuid -> Varchar,
        managed -> Nullable<Int4>,
    }
}

joinable!(ipaddresses -> managedusers (user_id));
joinable!(managedusers -> shadow (shadow));
joinable!(users -> managedusers (managed));

allow_tables_to_appear_in_same_query!(ipaddresses, managedusers, shadow, users,);
