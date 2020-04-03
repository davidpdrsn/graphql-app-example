pub mod pagination;

use crate::graphql::Context;
use crate::schema::*;
use diesel::prelude::*;
use juniper_eager_loading::impl_load_from_for_diesel_pg;

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

impl_load_from_for_diesel_pg! {
    (
        error = diesel::result::Error,
        context = Context,
    ) => {
        i32 -> (users, User),
        i32 -> (countries, Country),
    }
}
