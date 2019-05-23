use crate::{models, DbCon, DbConPool};
use diesel::prelude::*;
use juniper::{Executor, FieldResult};
use juniper_eager_loading::{prelude::*, Cache, DbEdge, EagerLoading, OptionDbEdge, VecDbEdge};
use juniper_from_schema::graphql_schema_from_file;
use rocket::{
    http::Status,
    request::{self, FromRequest, Request},
    Outcome, State,
};

graphql_schema_from_file!("schema.graphql");

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

pub struct Query;

impl QueryFields for Query {
    fn field_users(
        &self,
        executor: &Executor<'_, Context>,
        trail: &QueryTrail<'_, User, Walked>,
    ) -> FieldResult<Vec<User>> {
        use crate::schema::users;
        let con = &executor.context().db_con;

        let user_models = users::table.load::<models::User>(con)?;
        let mut users = User::from_db_models(&user_models);
        let mut cache = Cache::new();
        User::eager_load_all_children_for_each(&mut users, &user_models, con, trail, &mut cache)?;

        Ok(users)
    }
}

pub struct Mutation;

impl MutationFields for Mutation {
    fn field_noop(&self, _executor: &Executor<'_, Context>) -> FieldResult<&bool> {
        Ok(&true)
    }
}

#[derive(Clone, Debug, EagerLoading)]
#[eager_loading(
    model = "models::User",
    error = "diesel::result::Error",
    connection = "PgConnection"
)]
pub struct User {
    user: models::User,
    #[eager_loading(foreign_key_field = "country_id", model = "models::Country")]
    country: DbEdge<Country>,
}

#[derive(Clone, Debug, EagerLoading)]
#[eager_loading(
    model = "models::Country",
    error = "diesel::result::Error",
    connection = "PgConnection"
)]
pub struct Country {
    country: models::Country,
}

impl UserFields for User {
    fn field_id(&self, _executor: &Executor<'_, Context>) -> FieldResult<&i32> {
        Ok(&self.user.id)
    }

    fn field_name(&self, _executor: &Executor<'_, Context>) -> FieldResult<&String> {
        Ok(&self.user.name)
    }

    fn field_country(
        &self,
        executor: &Executor<'_, Context>,
        _trail: &QueryTrail<'_, Country, Walked>,
    ) -> FieldResult<&Country> {
        Ok(self.country.try_unwrap()?)
    }
}

impl CountryFields for Country {
    fn field_id(&self, _executor: &Executor<'_, Context>) -> FieldResult<&i32> {
        Ok(&self.country.id)
    }

    fn field_name(&self, _executor: &Executor<'_, Context>) -> FieldResult<&String> {
        Ok(&self.country.name)
    }
}
