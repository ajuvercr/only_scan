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

fn add<T, R: Into<T>>(to: &mut Vec<T>, t: R) {
    to.push(t.into());
}

fn remove<T: 'static, R: 'static + ?Sized>(to: &mut Vec<T>, t: &R)
where
    T: std::cmp::PartialEq<R>,
{
    to.retain(|x| !x.eq(t));
}

pub type Tasks = HashMap<i32, Task>;

pub fn get_tasks(conn: &mut Conn) -> QueryResult<Tasks> {
    let vec = Task::table().get_all(conn)?;
    Ok(vec.into_iter().map(|t| (t.id, t)).collect::<Tasks>())
}

impl Task {
    pub fn table() -> DbRepo<Self, tasks::table> {
        DbRepo::new()
    }

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
    Task::table().insert_one(task_new, &mut db_conn).ok()?;
    Redirect::to("/scrum").into()
}

#[get("/<id>")]
fn get_one(mut db_conn: DbConn, id: i32) -> Option<Template> {
    let tasks = get_tasks(&mut db_conn).ok()?;
    let task = Task::table().get_by_id(id, &mut db_conn).ok()?;
    Template::render("scrum/detail", task.to_value(&tasks)).into()
}

#[post("/<id>/sub/<child>")]
fn add_child(tasks: &State<Repo<Tasks>>, id: i32, child: i32) -> Option<()> {
    todo!()
    //tasks.with_save(|tasks| tasks.get_mut(&id).map(|t| add(&mut t.children, child)))
}

#[delete("/<id>/sub/<child>")]
fn remove_child(tasks: &State<Repo<Tasks>>, id: i32, child: i32) -> Option<()> {
    todo!()
    //tasks.with_save(|tasks| tasks.get_mut(&id).map(|t| remove(&mut t.children, &child)))
}

#[patch("/<id>", data = "<update>")]
fn patch_child(mut db_conn: DbConn, id: i32, update: Json<TaskBuilder>) -> Option<()> {
use crate::diesel::RunQueryDsl;
use crate::diesel::QueryDsl;

use std::ops::DerefMut;
    let update = update.into_inner();
    println!("patch to {} with {:?}", id, update);
        let find: u32 = tasks::table.find(id);
        diesel::update(find).set(update).execute( db_conn.deref_mut());
        Task::table().update_by_id(id, update, &mut db_conn).ok()?;
    Some(())
}

#[get("/")]
fn get(tasks: &State<Repo<Tasks>>) -> Template {
    let ctx = tasks.with(|tasks| {
        let tasks: Vec<_> = tasks
            .values()
            .filter(|x| x.parent.is_none())
            .flat_map(|t| t.to_value(tasks))
            .collect();
        rocket::serde::json::serde_json::json!({ "tasks": tasks })
    });

    Template::render("scrum/index", ctx)
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount(
            "/scrum",
            routes![
                get,
                get_one,
                create_one,
                patch_child,
                remove_child,
                add_child
            ],
        )
        .attach(AdHoc::config::<ScrumConfig>())
        .attach(Repo::<Tasks>::adhoc(
            "scrum",
            |c: &ScrumConfig| c.scrum_location.to_string(),
            HashMap::new(),
        ))
}
