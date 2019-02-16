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
    Outcome, State,
};

graphql_schema_from_file!("schema.graphql");

mod schema {
    table! {
        users (id) {
            id -> Integer,
            name -> Text,
        }
    }
}

type DbConPool = r2d2::Pool<ConnectionManager<PgConnection>>;
type DbCon = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub struct Context {
    pub db: DbCon,
}

impl juniper::Context for Context {}

impl<'a, 'r> FromRequest<'a, 'r> for Context {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Context, ()> {
        let db_pool = request.guard::<State<DbConPool>>()?;

        match db_pool.get() {
            Ok(db) => Outcome::Success(Context { db }),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub name: String,
}

impl UserFields for User {
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
        let db = &executor.context().db;
        let all_users = users::table.load::<User>(db)?;
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
    let database_url = "postgres://localhost/graphql-app-example";
    let db_pool = r2d2::Pool::builder()
        .max_size(10)
        .build(ConnectionManager::<PgConnection>::new(database_url))
        .expect("failed to create db connection pool");

    rocket::ignite()
        .manage(db_pool)
        .manage(Schema::new(Query, Mutation))
        .mount(
            "/",
            routes![graphiql, get_graphql_handler, post_graphql_handler],
        )
        .launch();
}
