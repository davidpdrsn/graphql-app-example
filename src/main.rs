#![feature(proc_macro_hygiene, decl_macro)]
#![deny(unused_imports, dead_code, unused_variables)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket_contrib;

mod graphql;
mod models;
mod schema;

#[cfg(test)]
mod tests;

use crate::graphql::*;
use rocket::{response::content, Rocket, State};

#[cfg(not(test))]
#[database("master")]
struct DbCon(diesel::PgConnection);

#[cfg(test)]
#[database("test")]
struct DbCon(diesel::PgConnection);

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
    rocket().launch();
}

fn rocket() -> Rocket {
    rocket::ignite()
        .manage(Schema::new(Query, Mutation))
        .mount(
            "/",
            routes![graphiql, get_graphql_handler, post_graphql_handler],
        )
        .attach(DbCon::fairing())
}
