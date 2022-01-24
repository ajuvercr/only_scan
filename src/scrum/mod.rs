use std::collections::HashMap;

use diesel::QueryResult;
use rocket::fairing::AdHoc;
use rocket::response::Redirect;
use rocket::serde::json::{serde_json, Json, Value};
use rocket::serde::Deserialize;
use rocket::{Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::repository::db_repo::Repo as DbRepo;
use crate::repository::Repository as Repo;
use crate::{Conn, DbConn};
pub mod models;
use models::{Task, TaskBuilder, TaskNew};

use self::models::tasks;

pub type Tasks = HashMap<i32, Task>;

pub fn get_tasks(conn: &mut Conn) -> QueryResult<Tasks> {
    let vec = TASK_TABLE.get_all(conn)?;
    Ok(vec.into_iter().map(|t| (t.id, t)).collect::<Tasks>())
}

pub const TASK_TABLE: DbRepo<Task, tasks::table> = DbRepo::new_t(tasks::table);

impl Task {
    pub fn get_points(&self, tasks: &Tasks) -> (i32, i32) {
        let mut out = (0, 0);

        if self.done {
            out.0 += self.points;
        }
        out.1 += self.points;

        for s in &self.children {
            let (x, y) = tasks[s].get_points(tasks);
            out.0 += x;
            out.1 += y;
        }

        out
    }

    pub fn to_value(&self, tasks: &Tasks) -> Option<Value> {
        let mut out = match serde_json::to_value(self) {
            Ok(Value::Object(x)) => x,
            _ => return None,
        };

        let value_tasks = self
            .children
            .iter()
            .flat_map(|t| tasks[t].to_value(tasks))
            .collect::<Vec<_>>();
        out.insert("sub_tasks".to_string(), Value::Array(value_tasks));

        let (done, total) = self.get_points(tasks);
        out.insert(String::from("completed"), Value::Number(done.into()));
        out.insert(String::from("total"), Value::Number(total.into()));

        if total == 0 {
            out.insert(String::from("progress"), Value::Number(0.into()));
        } else {
            out.insert(
                String::from("progress"),
                Value::Number((done * 100 / total).into()),
            );
        }
        Some(Value::Object(out))
    }
}

#[derive(Deserialize, Debug)]
struct ScrumConfig {
    #[serde(default = "default_location")]
    scrum_location: String,
}
fn default_location() -> String {
    "scrum.json".to_string()
}

#[post("/new")]
fn create_one(mut db_conn: DbConn) -> Option<Redirect> {
    let task_new = TaskNew {
        title: String::from("new"),
    };
    TASK_TABLE.insert_one(task_new, &mut db_conn).ok()?;
    Redirect::to("/scrum").into()
}

#[get("/<id>")]
fn get_one(mut db_conn: DbConn, id: i32) -> Option<Template> {
    let tasks = get_tasks(&mut db_conn).ok()?;
    let task = TASK_TABLE.get_by_id(id, &mut db_conn).ok()?;
    Template::render("scrum/detail", task.to_value(&tasks)).into()
}

#[post("/<id>?<new_parent>&<old_parent>")]
fn swap_parent(
    mut db_conn: DbConn,
    id: i32,
    new_parent: Option<i32>,
    old_parent: Option<i32>,
) -> Option<()> {
    if let Some(np) = new_parent {
        let mut children = TASK_TABLE.get_by_id(np, &mut db_conn).ok()?.children;
        children.push(id);
        TASK_TABLE
            .update_by_id(np, Task::builder().with_children(children), &mut db_conn)
            .ok()?;
    }
    if let Some(op) = old_parent {
        let mut children = TASK_TABLE.get_by_id(op, &mut db_conn).ok()?.children;
        children.retain(|&x| x != id);
        TASK_TABLE
            .update_by_id(op, Task::builder().with_children(children), &mut db_conn)
            .ok()?;
    }

    TASK_TABLE
        .update_by_id(id, Task::builder().with_parent(new_parent), &mut db_conn)
        .ok()?;

    Some(())
}

#[patch("/<id>", data = "<update>")]
fn patch_child(mut db_conn: DbConn, id: i32, update: Json<TaskBuilder>) -> Option<()> {
    let update = update.into_inner();
    TASK_TABLE.update_by_id(id, update, &mut db_conn).ok()?;
    Some(())
}

#[get("/")]
fn get(mut db_conn: DbConn) -> Option<Template> {
    let tasks = get_tasks(&mut db_conn).ok()?;
    let tasks = tasks
        .values()
        .filter(|x| x.parent.is_none())
        .flat_map(|t: &Task| t.to_value(&tasks))
        .collect::<Vec<_>>();
    let ctx = rocket::serde::json::serde_json::json!({ "tasks": tasks });

    Some(Template::render("scrum/index", ctx))
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount(
            "/scrum",
            routes![get, get_one, create_one, patch_child, swap_parent,],
        )
        .attach(AdHoc::config::<ScrumConfig>())
        .attach(Repo::<Tasks>::adhoc(
            "scrum",
            |c: &ScrumConfig| c.scrum_location.to_string(),
            HashMap::new(),
        ))
}
