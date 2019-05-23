#[derive(Queryable, Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub country_id: i32,
}

#[derive(Queryable, Clone)]
pub struct Country {
    pub id: i32,
    pub name: String,
}
