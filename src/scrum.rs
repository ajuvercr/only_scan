use std::collections::{HashMap, HashSet};

use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::response::Redirect;
use rocket::serde::json::{json, serde_json, Json, Value};
use rocket::serde::{Deserialize, Serialize};
use rocket::{Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::repository::Repository as Repo;
use crate::util::{self, id};

#[derive(Builder, Deserialize, Serialize, Debug, Clone)]
#[inner(FromForm, Deserialize, Serialize, Debug)]
struct Task {
    #[no_builder]
    id: String,
    done: bool,
    img: Option<String>,
    title: String,
    description: String,
    points: usize,
    parent: Option<String>,
    sub_tasks: Vec<String>,
}

fn add<T, R: Into<T>>(to: &mut Vec<T>, t: R) {
    to.push(t.into());
}

fn remove<T: 'static, R: 'static + ?Sized>(to: &mut Vec<T>, t: &R)
where
    T: std::cmp::PartialEq<R>,
{
    to.retain(|x| x.eq(t));
}

pub type Tasks = HashMap<String, Task>;

impl Task {
    pub fn new() -> Self {
        Self {
            id: id(),
            done: false,
            img: None,
            title: String::from("Some title"),
            description: String::from("Some description"),
            points: 0,
            parent: None,
            sub_tasks: Vec::new(),
        }
    }

    pub fn get_points(&self, tasks: &Tasks) -> (usize, usize) {
        let mut out = (0, 0);

        if self.done {
            out.0 += self.points;
        }
        out.1 += self.points;

        for s in &self.sub_tasks {
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
            .sub_tasks
            .iter()
            .flat_map(|t| tasks[t].to_value(tasks))
            .collect::<Vec<_>>();
        out.insert("sub_tasks".to_string(), Value::Array(value_tasks));

        let (done, total) = self.get_points(tasks);
        out.insert(String::from("done"), Value::Number(done.into()));
        out.insert(String::from("total"), Value::Number(total.into()));

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
fn create_one(tasks: &State<Repo<Tasks>>) -> Redirect {
    tasks.with_save(|tasks| {
        let new = Task::new();
        let id = new.id.clone();
        tasks.insert(id, new);
    });
    Redirect::to("/scrum")
}

#[get("/<id>")]
fn get_one(tasks: &State<Repo<Tasks>>, id: &str) -> Template {
    let ctx = tasks.with(|tasks| tasks[id].to_value(tasks));
    println!("ctx {:?}", ctx);
    Template::render("scrum/detail", ctx)
}

#[post("/<id>/sub/<child>")]
fn add_child(tasks: &State<Repo<Tasks>>, id: &str, child: &str) -> Redirect {
    let ctx = tasks.with_save(|tasks| {
        tasks.get_mut(id).map(|t| add(&mut t.sub_tasks, child));
    });
    Redirect::to("/scrum")
}

#[delete("/<id>/sub/<child>")]
fn remove_child(tasks: &State<Repo<Tasks>>, id: &str, child: &str) -> Redirect {
    let ctx = tasks.with_save(|tasks| tasks.get_mut(id).map(|t| remove(&mut t.sub_tasks, child)));
    Redirect::to("/scrum")
}

#[patch("/<id>", data = "<update>")]
fn patch_child(tasks: &State<Repo<Tasks>>, id: &str, update: Json<TaskBuilder>) -> Redirect {
    let update = update.into_inner();
    println!("patch to {} with {:?}", id, update);
    tasks.with_save(|tasks| tasks.get_mut(id).map(|t| t.update(update)));
    Redirect::to("/scrum")
}

#[get("/")]
fn get(tasks: &State<Repo<Tasks>>) -> Template {
    let ctx = tasks.with(|tasks| {
        let tasks: Vec<_> = tasks.values().filter(|x| x.parent.is_none()).flat_map(|t| t.to_value(tasks)).collect();
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
