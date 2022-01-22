use rocket::serde::{Deserialize, Serialize};
use crate::Conn;

table! {
    tasks (id) {
        id -> Int4,
        done -> Bool,
        img -> Nullable<Varchar>,
        title -> Varchar,
        description -> Varchar,
        points -> Int4,
        parent -> Nullable<Varchar>,
        children -> Array<Int4>,
    }
}

#[derive(Queryable, Insertable, Builder, Deserialize, Serialize, Debug, Clone)]
#[inner(#[derive(FromForm, Deserialize, Serialize, Debug, AsChangeset, Clone)] #[table_name = "tasks"])]
pub struct Task {
    #[no_builder]
    pub id: i32,
    pub done: bool,
    pub img: Option<String>,
    pub title: String,
    pub description: String,
    pub points: i32,
    pub parent: Option<String>,
    pub children: Vec<i32>,
}

use diesel::prelude::*;
impl Task {
    fn get_by_id(id: i32, conn: &mut Conn) -> Option<Self> {
        tasks::table.find(id).first::<Task>(conn).ok()
    }

    fn update_by_id(id: i32, update: TaskBuilder, conn: &mut Conn) -> Option<usize> {
        diesel::update(tasks::table.find(id)).set(update).execute(conn).ok()
        // diesel::update(tasks::table).filter(tasks::id.eq(id)).set(update).execute(conn).ok()
    }

}
