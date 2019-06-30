use crate::{models, DbCon, DbConPool};
use diesel::prelude::*;
use juniper::{Executor, FieldResult};
use juniper_eager_loading::{prelude::*, *};
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
        let users = map_models_to_graphql_nodes(&user_models, &trail, con)?;

        Ok(users)
    }

    fn field_user_connections(
        &self,
        executor: &Executor<'_, Context>,
        trail: &QueryTrail<'_, UserConnection, Walked>,
        after: Option<Cursor>,
        first: i32,
        user_id: i32,
    ) -> FieldResult<Option<UserConnection>> {
        use crate::schema::users;
        let con = &executor.context().db_con;

        let user = users::table
            .filter(users::id.eq(user_id))
            .first::<models::User>(con)
            .optional()?;

        let user_connection = if let Some(user) = user {
            Some(user_connections(user, after, first, trail, con)?)
        } else {
            None
        };

        Ok(user_connection)
    }
}

fn user_connections(
    user: models::User,
    cursor: Option<Cursor>,
    page_size: i32,
    trail: &QueryTrail<'_, UserConnection, Walked>,
    con: &PgConnection,
) -> QueryResult<UserConnection> {
    use crate::{models::pagination::*, schema::users};

    let page_size = i64::from(page_size);

    let page_number = cursor
        .unwrap_or_else(|| Cursor("1".to_string()))
        .0
        .parse::<i64>()
        .expect("invalid cursor");
    let next_page_cursor = Cursor((page_number + 1).to_string());

    let (user_models, total_count) = users::table
        .select(users::all_columns)
        .paginate(page_number)
        .per_page(page_size)
        .load_and_count_pages::<models::User>(con)?;

    let users = if let Some(user_trail) = trail.edges().node().walk() {
        map_models_to_graphql_nodes(&user_models, &user_trail, con)?
    } else {
        vec![]
    };

    let edges = users
        .into_iter()
        .map(|user| Edge {
            node: user,
            cursor: next_page_cursor.clone(),
        })
        .collect::<Vec<_>>();

    // TODO
    let page_info = PageInfo {
        start_cursor: edges.first().map(|edge| edge.cursor.clone()),
        end_cursor: edges.last().map(|edge| edge.cursor.clone()),
        has_next_page: false,
    };

    Ok(UserConnection {
        edges,
        page_info,
        total_count: total_count as i32,
    })
}

use juniper_eager_loading::{EagerLoadAllChildren, GraphqlNodeForModel};

fn map_models_to_graphql_nodes<'a, T, M: Clone>(
    models: &[M],
    trail: &QueryTrail<'a, T, Walked>,
    con: &PgConnection,
) -> Result<Vec<T>, diesel::result::Error>
where
    T: EagerLoadAllChildren<QueryTrail<'a, T, Walked>>
        + GraphqlNodeForModel<Model = M, Connection = PgConnection, Error = diesel::result::Error>,
{
    let mut users = T::from_db_models(models);
    T::eager_load_all_children_for_each(&mut users, models, con, trail)?;
    Ok(users)
}

impl Clone for Cursor {
    fn clone(&self) -> Self {
        Cursor(self.0.clone())
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
    #[has_one(default)]
    country: HasOne<Country>,
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
    fn field_id(&self, _: &Executor<'_, Context>) -> FieldResult<&i32> {
        Ok(&self.user.id)
    }

    fn field_name(&self, _: &Executor<'_, Context>) -> FieldResult<&String> {
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

pub struct PageInfo {
    start_cursor: Option<Cursor>,
    end_cursor: Option<Cursor>,
    has_next_page: bool,
}

impl PageInfoFields for PageInfo {
    fn field_start_cursor(&self, _: &Executor<'_, Context>) -> FieldResult<&Option<Cursor>> {
        Ok(&self.start_cursor)
    }

    fn field_end_cursor(&self, _: &Executor<'_, Context>) -> FieldResult<&Option<Cursor>> {
        Ok(&self.end_cursor)
    }

    fn field_has_next_page(&self, _: &Executor<'_, Context>) -> FieldResult<&bool> {
        Ok(&self.has_next_page)
    }
}

pub struct UserConnection {
    edges: Vec<UserEdge>,
    page_info: PageInfo,
    total_count: i32,
}

impl UserConnectionFields for UserConnection {
    fn field_edges(
        &self,
        _: &Executor<'_, Context>,
        _: &QueryTrail<'_, UserEdge, Walked>,
    ) -> FieldResult<&Vec<UserEdge>> {
        Ok(&self.edges)
    }

    fn field_page_info(
        &self,
        _: &Executor<'_, Context>,
        _: &QueryTrail<'_, PageInfo, Walked>,
    ) -> FieldResult<&PageInfo> {
        Ok(&self.page_info)
    }

    fn field_total_count(&self, _: &Executor<'_, Context>) -> FieldResult<&i32> {
        Ok(&self.total_count)
    }
}

pub struct Edge<T> {
    node: T,
    cursor: Cursor,
}

pub type UserEdge = Edge<User>;

impl UserEdgeFields for UserEdge {
    fn field_node(
        &self,
        _: &Executor<'_, Context>,
        _: &QueryTrail<'_, User, Walked>,
    ) -> FieldResult<&User> {
        Ok(&self.node)
    }

    fn field_cursor(&self, _: &Executor<'_, Context>) -> FieldResult<&Cursor> {
        Ok(&self.cursor)
    }
}
