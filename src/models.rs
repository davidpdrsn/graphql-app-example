pub mod pagination;

use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_dsl::methods::LoadQuery;
use diesel::sql_types::BigInt;
use juniper_eager_loading::impl_LoadFrom_for_diesel;
use crate::schema::*;

#[derive(Queryable, Debug, Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub country_id: i32,
}

#[derive(Queryable, Debug, Clone)]
pub struct Country {
    pub id: i32,
    pub name: String,
}

impl_LoadFrom_for_diesel! {
    (
        error = diesel::result::Error,
        connection = PgConnection,
    ) => {
        i32 -> (users, User),
        i32 -> (countries, Country),
    }
}
