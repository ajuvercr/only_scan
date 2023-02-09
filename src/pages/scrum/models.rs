use crud_helper::{Builder, New};
use diesel::{table, AsChangeset, Insertable, Queryable};
use rocket::serde::{Deserialize, Serialize};

table! {
    tasks (id) {
        id -> Int4,
        done -> Bool,
        img -> Nullable<Varchar>,
        title -> Varchar,
        description -> Varchar,
        points -> Int4,
        parent -> Nullable<Int4>,
        children -> Array<Int4>,
    }
}
#[derive(Queryable, Builder, New, Deserialize, Serialize, Debug, Clone)]
#[inner_builder(#[derive(FromForm, Deserialize, Serialize, Debug, AsChangeset, Clone)] #[table_name = "tasks"])]
#[inner_new(#[derive(FromForm, Queryable, Deserialize, Serialize, Debug, Insertable, Clone)] #[table_name = "tasks"])]
pub struct Task {
    #[no_builder]
    #[no_new]
    pub id: i32,
    #[no_new]
    pub done: bool,
    #[no_new]
    pub img: Option<String>,
    pub title: String,
    #[no_new]
    pub description: String,
    #[no_new]
    pub points: i32,
    #[no_new]
    pub parent: Option<i32>,
    #[no_new]
    pub children: Vec<i32>,
}
