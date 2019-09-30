#![feature(proc_macro_hygiene, decl_macro)]
#![deny(unused_imports, dead_code, unused_variables)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

mod graphql;
mod models;
mod schema;

#[cfg(test)]
mod tests;

use crate::graphql::*;
use diesel::{prelude::*, r2d2::ConnectionManager};
use rocket::{response::content, Rocket, State};

type DbConPool = r2d2::Pool<ConnectionManager<PgConnection>>;
type DbCon = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

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
    dotenv::dotenv().ok();
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
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    r2d2::Pool::builder()
        .max_size(10)
        .build(ConnectionManager::<PgConnection>::new(database_url))
        .expect("failed to create db connection pool")
}
