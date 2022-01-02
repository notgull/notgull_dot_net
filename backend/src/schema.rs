table! {
    blogposts (id) {
        id -> Int4,
        title -> Varchar,
        tags -> Varchar,
        url -> Varchar,
        body -> Text,
        author_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Int4,
        uuid -> Varchar,
        name -> Nullable<Varchar>,
        roles -> Int8,
    }
}

joinable!(blogposts -> users (author_id));

allow_tables_to_appear_in_same_query!(blogposts, users,);
