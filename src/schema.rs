table! {
    users (id) {
        id -> Integer,
        name -> Text,
        country_id -> Integer,
    }
}

table! {
    countries (id) {
        id -> Integer,
        name -> Text,
    }
}
