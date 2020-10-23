#![allow(unused_braces, clippy::unnecessary_lazy_evaluations)]

use crate::{
    models::{Country, User},
    DbCon,
};
use diesel::prelude::*;
use juniper::{Executor, FieldResult, ID};
// use juniper_eager_loading::{prelude::*, *};
// use juniper_eager_loading::{EagerLoadAllChildren, GraphqlNodeForModel};
use juniper_from_schema::graphql_schema_from_file;
use rocket::{
    request::{self, FromRequest, Request},
    Outcome,
};
use std::sync::{Arc, Mutex};

graphql_schema_from_file!("schema.graphql");

pub struct Context {
    db_con: Arc<Mutex<DbCon>>,
}

impl juniper::Context for Context {}

impl<'a, 'r> FromRequest<'a, 'r> for Context {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Context, ()> {
        let db_con = request.guard::<DbCon>()?;
        Outcome::Success(Context {
            db_con: Arc::new(Mutex::new(db_con)),
        })
    }
}

pub struct Query;

impl QueryFields for Query {
    fn field_users(
        &self,
        executor: &Executor<Context>,
        _trail: &QueryTrail<User, Walked>,
    ) -> FieldResult<Vec<User>> {
        use crate::schema::users;
        let ctx = &executor.context();
        let con = ctx.db_con.lock().unwrap();

        let users = users::table.load::<User>(&**con)?;
        // let users = map_models_to_graphql_nodes(&user_models, &trail, ctx)?;

        Ok(users)
    }

    fn field_user_connections(
        &self,
        executor: &Executor<Context>,
        trail: &QueryTrail<UserConnection, Walked>,
        after: Option<Cursor>,
        first: i32,
    ) -> FieldResult<UserConnection> {
        let ctx = &executor.context();
        let user_connection = user_connections(after, first, trail, ctx)?;
        Ok(user_connection)
    }
}

fn user_connections(
    cursor: Option<Cursor>,
    page_size: i32,
    _trail: &QueryTrail<'_, UserConnection, Walked>,
    ctx: &Context,
) -> QueryResult<UserConnection> {
    use crate::{models::pagination::*, schema::users};

    let con = ctx.db_con.lock().unwrap();

    let page_size = i64::from(page_size);

    let page_number = cursor
        .unwrap_or_else(|| Cursor("1".to_string()))
        .0
        .parse::<i64>()
        .expect("invalid cursor");
    let next_page_cursor = Cursor((page_number + 1).to_string());

    let base_query = users::table.select(users::all_columns).order(users::id);

    let (users, total_count) = base_query
        .paginate(page_number)
        .per_page(page_size)
        .load_and_count_pages::<User>(&**con)?;

    // let users = if let Some(user_trail) = trail.edges().node().walk() {
    //     map_models_to_graphql_nodes(&user_models, &user_trail, ctx)?
    // } else {
    //     vec![]
    // };

    let edges = users
        .into_iter()
        .map(|user| Edge {
            node: user,
            cursor: next_page_cursor.clone(),
        })
        .collect::<Vec<_>>();

    let page_info = PageInfo {
        start_cursor: edges.first().map(|edge| edge.cursor.clone()),
        end_cursor: edges.last().map(|edge| edge.cursor.clone()),
        has_next_page: {
            let next_page = base_query
                .paginate(page_number + 1)
                .per_page(1)
                .load::<(User, i64)>(&**con)?;
            !next_page.is_empty()
        },
    };

    Ok(UserConnection {
        edges,
        page_info,
        total_count: total_count as i32,
    })
}

// fn map_models_to_graphql_nodes<'a, T, M: Clone>(
//     models: &[M],
//     trail: &QueryTrail<'a, T, Walked>,
//     ctx: &Context,
// ) -> Result<Vec<T>, diesel::result::Error>
// where
//     T: EagerLoadAllChildren
//         + GraphqlNodeForModel<Model = M, Context = Context, Error = diesel::result::Error>,
// {
//     let mut users = T::from_db_models(models);
//     T::eager_load_all_children_for_each(&mut users, models, ctx, trail)?;
//     Ok(users)
// }

pub struct Mutation;

impl MutationFields for Mutation {
    fn field_noop(&self, _executor: &Executor<Context>) -> FieldResult<&bool> {
        Ok(&true)
    }
}

impl UserFields for User {
    fn field_id(&self, _: &Executor<Context>) -> FieldResult<ID> {
        Ok(ID::new(self.id.to_string()))
    }

    fn field_name(&self, _: &Executor<Context>) -> FieldResult<&String> {
        Ok(&self.name)
    }

    fn field_country(
        &self,
        executor: &Executor<Context>,
        _trail: &QueryTrail<Country, Walked>,
    ) -> FieldResult<Country> {
        use crate::schema::countries;
        let ctx = &executor.context();
        let con = ctx.db_con.lock().unwrap();

        let country = countries::table
            .filter(countries::id.eq(self.country_id))
            .first::<Country>(&**con)?;

        Ok(country)
    }
}

impl CountryFields for Country {
    fn field_id(&self, _executor: &Executor<Context>) -> FieldResult<ID> {
        Ok(ID::new(format!("{}", self.id)))
    }

    fn field_name(&self, _executor: &Executor<Context>) -> FieldResult<&String> {
        Ok(&self.name)
    }
}

pub struct PageInfo {
    start_cursor: Option<Cursor>,
    end_cursor: Option<Cursor>,
    has_next_page: bool,
}

impl PageInfoFields for PageInfo {
    fn field_start_cursor(&self, _: &Executor<Context>) -> FieldResult<&Option<Cursor>> {
        Ok(&self.start_cursor)
    }

    fn field_end_cursor(&self, _: &Executor<Context>) -> FieldResult<&Option<Cursor>> {
        Ok(&self.end_cursor)
    }

    fn field_has_next_page(&self, _: &Executor<Context>) -> FieldResult<&bool> {
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
        _: &Executor<Context>,
        _: &QueryTrail<UserEdge, Walked>,
    ) -> FieldResult<&Vec<UserEdge>> {
        Ok(&self.edges)
    }

    fn field_page_info(
        &self,
        _: &Executor<Context>,
        _: &QueryTrail<PageInfo, Walked>,
    ) -> FieldResult<&PageInfo> {
        Ok(&self.page_info)
    }

    fn field_total_count(&self, _: &Executor<Context>) -> FieldResult<&i32> {
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
        _: &Executor<Context>,
        _: &QueryTrail<User, Walked>,
    ) -> FieldResult<&User> {
        Ok(&self.node)
    }

    fn field_cursor(&self, _: &Executor<Context>) -> FieldResult<&Cursor> {
        Ok(&self.cursor)
    }
}
