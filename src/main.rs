#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

use diesel::{pg::PgConnection, prelude::*, r2d2::ConnectionManager};
use juniper::{Executor, FieldResult};
use juniper_from_schema::graphql_schema_from_file;
use rocket::{
    http::Status,
    request::{self, FromRequest, Request},
    response::content,
    Outcome, Rocket, State,
};
use serde_json::{json, Value};

graphql_schema_from_file!("schema.graphql");

mod schema {
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
}

type DbConPool = r2d2::Pool<ConnectionManager<PgConnection>>;
type DbCon = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub struct Context {
    pub db_con: DbCon,
}

impl juniper::Context for Context {}

impl<'a, 'r> FromRequest<'a, 'r> for Context {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Context, ()> {
        let db_pool = request.guard::<State<DbConPool>>()?;

        match db_pool.get() {
            Ok(db_con) => Outcome::Success(Context { db_con }),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}

#[derive(Queryable, Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub country_id: i32,
}

impl UserFields for User {
    fn field_id(&self, _executor: &Executor<'_, Context>) -> FieldResult<&i32> {
        Ok(&self.id)
    }

    fn field_name(&self, _executor: &Executor<'_, Context>) -> FieldResult<&String> {
        Ok(&self.name)
    }

    fn field_country(
        &self,
        executor: &Executor<'_, Context>,
        _trail: &QueryTrail<'_, Country, Walked>,
    ) -> FieldResult<Country> {
        use crate::schema::countries;
        let con = &executor.context().db_con;
        let country = countries::table
            .filter(countries::id.eq(self.country_id))
            .first::<Country>(con)?;
        Ok(country)
    }
}

#[derive(Queryable, Clone)]
pub struct Country {
    pub id: i32,
    pub name: String,
}

impl CountryFields for Country {
    fn field_id(&self, _executor: &Executor<'_, Context>) -> FieldResult<&i32> {
        Ok(&self.id)
    }

    fn field_name(&self, _executor: &Executor<'_, Context>) -> FieldResult<&String> {
        Ok(&self.name)
    }
}

pub struct Query;

impl QueryFields for Query {
    fn field_users(
        &self,
        executor: &Executor<'_, Context>,
        _trail: &QueryTrail<'_, User, Walked>,
    ) -> FieldResult<Vec<User>> {
        use crate::schema::users;
        let con = &executor.context().db_con;
        let all_users = users::table.load::<User>(con)?;
        Ok(all_users)
    }
}

pub struct Mutation;

impl MutationFields for Mutation {
    fn field_noop(&self, _executor: &Executor<'_, Context>) -> FieldResult<&bool> {
        Ok(&true)
    }
}

#[get("/graphiql")]
fn graphiql() -> content::Html<String> {
    juniper_rocket::graphiql_source("/graphql")
}

#[get("/graphql?<request>")]
fn get_graphql_handler(
    context: Context,
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &context)
}

#[post("/graphql", data = "<request>")]
fn post_graphql_handler(
    context: Context,
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &context)
}

fn main() {
    rocket(db_pool()).launch();
}

fn rocket(db_pool: DbConPool) -> Rocket {
    rocket::ignite()
        .manage(db_pool)
        .manage(Schema::new(Query, Mutation))
        .mount(
            "/",
            routes![graphiql, get_graphql_handler, post_graphql_handler],
        )
}

fn db_pool() -> DbConPool {
    let database_url = "postgres://localhost/graphql-app-example";
    r2d2::Pool::builder()
        .max_size(10)
        .build(ConnectionManager::<PgConnection>::new(database_url))
        .expect("failed to create db connection pool")
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use super::*;
    use assert_json_diff::{assert_json_eq, assert_json_include};
    use diesel_factories::{Association, Factory};

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
        let database_url = "postgres://localhost/graphql-app-example";
        r2d2::Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<PgConnection>::new(database_url))
            .expect("failed to create db connection pool")
    }
}
