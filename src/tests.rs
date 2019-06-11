use crate::{models::*, rocket, DbConPool};
use assert_json_diff::{assert_json_eq, assert_json_include};
use diesel::{prelude::*, r2d2::ConnectionManager};
use diesel_factories::{Association, Factory};
use serde_json::{json, Value};

use rocket::{
    http::{ContentType, Status},
    local::Client,
};

#[test]
fn test_nothing_to_begin_with() {
    let (_pool, client) = setup();

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
    let (pool, client) = setup();

    let user = {
        let con = pool.get().unwrap();
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
                        "id": user.id,
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
    let (pool, client) = setup();

    let (user, country) = {
        let con = pool.get().unwrap();
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
                        "id": user.id,
                        "name": user.name,
                        "country": {
                            "id": country.id,
                            "name": country.name,
                        },
                    },
                ],
            },
        }),
        actual: json,
    );
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

fn setup() -> (DbConPool, Client) {
    let db_pool = test_db_pool();
    let con = db_pool.get().unwrap();
    con.begin_test_transaction().unwrap();

    let rocket = rocket(db_pool.clone());
    let client = Client::new(rocket).unwrap();

    (db_pool, client)
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

#[cfg(test)]
fn test_db_pool() -> DbConPool {
    let database_url = "postgres://localhost/graphql-app-example-test";
    r2d2::Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<PgConnection>::new(database_url))
        .expect("failed to create db connection pool")
}
