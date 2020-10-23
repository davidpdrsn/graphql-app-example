use crate::{models::*, DbCon};
use assert_json_diff::{assert_json_eq, assert_json_include};
use diesel::prelude::*;
use diesel_factories::{Association, Factory};
use juniper::ID;
use rocket::{
    http::{ContentType, Status},
    local::Client,
};
use serde_json::{json, Value};

#[test]
fn test_nothing_to_begin_with() {
    let client = setup();

    let query = "{ users { id name } }";

    let (json, status) = make_request(&client, query, None);

    assert_eq!(Status::Ok, status);
    assert_json_eq!(
        json!({
            "data": {
                "users": []
            }
        }),
        json,
    );
}

#[test]
fn test_loading_user() {
    let client = setup();

    let user = {
        let con = get_db_con(&client);
        UserFactory::default().insert(&con)
        // We need the connection to be dropped here for the rocket app to have access to it
        // because our pool size is 1. So if we held onto the connection it wouldn't work.
    };

    let query = r#"
        {
            users {
                id
                name
            }
        }
        "#;

    let (json, status) = make_request(&client, query, None);

    assert_eq!(Status::Ok, status);
    assert_json_include!(
        expected: json!({
            "data": {
                "users": [
                    {
                        "id": ID::new(user.id.to_string()),
                        "name": user.name,
                    },
                ],
            },
        }),
        actual: json,
    );
}

#[test]
fn test_loading_users_with_countries() {
    let client = setup();

    let (user, country) = {
        let con = get_db_con(&client);
        let country = CountryFactory::default().insert(&con);
        let user = UserFactory::default().country(&country).insert(&con);
        (user, country)
    };

    let query = r#"
        {
            users {
                id
                name
                country {
                    id
                    name
                }
            }
        }
        "#;

    let (json, status) = make_request(&client, query, None);

    assert_eq!(Status::Ok, status);
    assert_json_include!(
        expected: json!({
            "data": {
                "users": [
                    {
                        "id": ID::new(user.id.to_string()),
                        "name": user.name,
                        "country": {
                            "id": ID::new(country.id.to_string()),
                            "name": country.name,
                        },
                    },
                ],
            },
        }),
        actual: json,
    );
}

#[test]
fn test_paginating_users() {
    let client = setup();

    let (user_1, user_2, user_3) = {
        let con = get_db_con(&client);
        let user_1 = UserFactory::default().name("1").insert(&con);
        let user_2 = UserFactory::default().name("2").insert(&con);
        let user_3 = UserFactory::default().name("3").insert(&con);
        (user_1, user_2, user_3)
    };

    let query = r#"
        {
            userConnections(first: 1) {
                edges {
                    cursor
                    node {
                        id
                        name
                    }
                }
                pageInfo {
                    startCursor
                    endCursor
                    hasNextPage
                }
                totalCount
            }
        }
        "#;
    let (json, status) = make_request(&client, query, None);

    assert_eq!(Status::Ok, status);
    assert_json_include!(
        expected: json!({
            "data": {
                "userConnections": {
                    "edges": [
                        {
                            "cursor": "2",
                            "node": { "name": user_1.name },
                        }
                    ],
                    "pageInfo": {
                        "startCursor": "2",
                        "endCursor": "2",
                        "hasNextPage": true,
                    },
                    "totalCount": 3,
                }
            },
        }),
        actual: json.clone(),
    );
    let cursor = json["data"]["userConnections"]["pageInfo"]["endCursor"]
        .as_str()
        .unwrap();

    let query = r#"
        query Test($after: Cursor!) {
            userConnections(first: 1, after: $after) {
                edges {
                    cursor
                    node {
                        id
                        name
                    }
                }
                pageInfo {
                    startCursor
                    endCursor
                    hasNextPage
                }
                totalCount
            }
        }
        "#;
    let vars = json!({ "after": cursor });
    let (json, status) = make_request(&client, query, Some(vars));

    assert_eq!(Status::Ok, status);
    assert_json_include!(
        expected: json!({
            "data": {
                "userConnections": {
                    "edges": [
                        {
                            "cursor": "3",
                            "node": { "name": user_2.name },
                        }
                    ],
                    "pageInfo": {
                        "startCursor": "3",
                        "endCursor": "3",
                        "hasNextPage": true,
                    },
                    "totalCount": 3,
                }
            },
        }),
        actual: json.clone(),
    );
    let cursor = json["data"]["userConnections"]["pageInfo"]["endCursor"]
        .as_str()
        .unwrap();

    let vars = json!({ "after": cursor });
    let (json, status) = make_request(&client, query, Some(vars));

    assert_eq!(Status::Ok, status);
    assert_json_include!(
        expected: json!({
            "data": {
                "userConnections": {
                    "edges": [
                        {
                            "cursor": "4",
                            "node": { "name": user_3.name },
                        }
                    ],
                    "pageInfo": {
                        "startCursor": "4",
                        "endCursor": "4",
                        "hasNextPage": false,
                    },
                    "totalCount": 3,
                }
            },
        }),
        actual: json.clone(),
    );
    let cursor = json["data"]["userConnections"]["pageInfo"]["endCursor"]
        .as_str()
        .unwrap();

    let vars = json!({ "after": cursor });
    let (json, status) = make_request(&client, query, Some(vars));
    assert_eq!(Status::Ok, status);
    let edges = json["data"]["userConnections"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 0);
}

#[test]
fn test_paginating_users_with_no_users() {
    let client = setup();

    let query = r#"
        {
            userConnections {
                edges {
                    cursor
                    node {
                        id
                    }
                }
                pageInfo {
                    startCursor
                    endCursor
                    hasNextPage
                }
            }
        }
        "#;

    let (json, status) = make_request(&client, query, None);

    assert_eq!(Status::Ok, status);
    assert_json_include!(
        expected: json!({
            "data": {
                "userConnections": {
                    "edges": [],
                    "pageInfo": {
                        "startCursor": null,
                        "endCursor": null,
                        "hasNextPage": false,
                    }
                }
            },
        }),
        actual: json.clone(),
    );
    let edges = json["data"]["userConnections"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 0);
}

#[derive(Clone, Factory)]
#[factory(
    model = "User",
    table = "crate::schema::users",
    connection = "PgConnection"
)]
struct UserFactory<'a> {
    pub name: String,
    pub country: Association<'a, Country, CountryFactory>,
}

impl Default for UserFactory<'_> {
    fn default() -> Self {
        Self {
            name: "Bob".to_string(),
            country: Association::default(),
        }
    }
}

#[derive(Clone, Factory)]
#[factory(
    model = "Country",
    table = "crate::schema::countries",
    connection = "PgConnection"
)]
struct CountryFactory {
    pub name: String,
}

impl Default for CountryFactory {
    fn default() -> Self {
        Self {
            name: "Copenhagen".to_string(),
        }
    }
}

fn setup() -> Client {
    let rocket = crate::rocket();
    let con = DbCon::get_one(&rocket).expect("get db con");
    con.begin_test_transaction()
        .expect("begin test transaction");
    drop(con);

    Client::new(rocket).expect("create test client")
}

fn get_db_con(client: &Client) -> DbCon {
    DbCon::get_one(client.rocket()).expect("get db con")
}

fn make_request(client: &Client, query: &str, variables: Option<Value>) -> (Value, Status) {
    let mut req = client.post("/graphql").header(ContentType::JSON);
    req.set_body(
        json!({
            "query": query,
            "variables": variables.unwrap_or_else(|| json!({})),
        })
        .to_string(),
    );

    let mut response = req.dispatch();
    let json = serde_json::from_str::<Value>(&response.body_string().unwrap()).unwrap();
    (json, response.status())
}
