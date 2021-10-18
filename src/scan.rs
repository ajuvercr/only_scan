use std::ops::Deref;
use std::sync::{Arc, Mutex};

use rocket::fairing::AdHoc;
use rocket::futures::future::BoxFuture;
use rocket::response::Redirect;
// use rocket::serde::json::json;
use rocket::serde::json::serde_json::{self, json};
use rocket::serde::{Deserialize, Serialize};
use rocket::{Build, Orbit, Rocket};
use rocket_dyn_templates::Template;
use std::fs;

use chrono::DateTime;

use crate::sorted_list::SortedList;
use crate::util::*;

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Scan {
    id: uuid::Uuid,
    date: DateTime<chrono::Utc>,
}

impl Scan {
    fn sorted_list() -> SortedList<Scan> {
        SortedList::new_on_field(|x: &Scan| x.date)
    }
}

struct ScanConfig {
    scans: SortedList<Scan>,
    location: String,
}

type ScanConfigState = Arc<Mutex<ScanConfig>>;
impl Default for ScanConfig {
    fn default() -> Self {
        ScanConfig {
            scans: Scan::sorted_list(),
            location: String::from("Desk.toml"),
        }
    }
}

impl ScanConfig {
    fn new(path: &str) -> Self {
        Self {
            scans: Scan::sorted_list(),
            location: path.to_string(),
        }
    }

    fn save(&self) -> Option<()> {
        let vec = serde_json::to_vec_pretty(&self.scans.deref()).ok()?;
        fs::write(&self.location, &vec).ok()
    }
}

#[derive(Deserialize, Debug)]
struct ScanConfigConfig {
    #[serde(default = "default_location")]
    scan_config_location: String,
}

fn default_location() -> String {
    "desk_config.json".to_string()
}

fn configure_desk<'a>(rocket: &'a Rocket<Orbit>) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        if let Some(ScanConfigConfig {
            scan_config_location,
            ..
        }) = rocket.state::<ScanConfigConfig>()
        {
            let config = initial_read_state(scan_config_location)
                .await
                .expect("Something failed");

            let mut desk_config = get_mutexed_rocket::<ScanConfig>(&rocket);
            *desk_config = config;
        }
    })
}

async fn initial_read_state(location: &str) -> Option<ScanConfig> {
    match read_file::<Vec<Scan>>(location).await {
        Some(scans) => Some(ScanConfig {
            scans: Scan::sorted_list().with_inner(scans),
            location: location.into(),
        }),
        None => {
            let out = ScanConfig::new(location);

            out.save()?;
            Some(out)
        }
    }
}

#[get("/")]
fn get() -> Template {
    let context = json!({
        "errors": [],
        "scans": [1,2,3,4,5]
    });

    Template::render("scan/index", &context)
}

#[get("/new")]
fn new_get() -> Template {
    let context = json!({
        "errors": []
    });

    Template::render("scan/new", &context)
}

#[post("/new")]
fn new_post() -> Redirect {
    Redirect::to("/scan")
}

#[get("/<uuid>")]
fn get_one(uuid: &str) -> Template {
    let context = json!({
        "errors": []
    });

    Template::render("scan/one", &context)
}

#[post("/<uuid>")]
fn post_one(uuid: &str) -> Redirect {
    Redirect::to(format!("/scan/{}", uuid))
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .manage(Arc::new(Mutex::new(ScanConfig::default())))
        .mount("/scan", routes![get, new_get, new_post, get_one, post_one])
        .attach(AdHoc::on_liftoff("configure desk", configure_desk))
        .attach(AdHoc::config::<ScanConfigConfig>())
}
