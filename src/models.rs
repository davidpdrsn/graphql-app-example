use diesel::prelude::*;
use juniper_eager_loading::diesel::LoadFromIds;

#[derive(Queryable, Debug, Clone, LoadFromIds)]
#[load_from_ids(table = "crate::schema::users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub country_id: i32,
}

#[derive(Queryable, Debug, Clone, LoadFromIds)]
#[load_from_ids(table = "crate::schema::countries")]
pub struct Country {
    pub id: i32,
    pub name: String,
}
