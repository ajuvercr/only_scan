use std::ops::Deref;
use std::sync::{Arc, Mutex};

use rocket::fairing::AdHoc;
use rocket::futures::future::BoxFuture;
use rocket::response::Redirect;
// use rocket::serde::json::json;
use rocket::form::Form;
use rocket::serde::json::serde_json::{self, json};
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket::{Build, Orbit, Rocket};
use rocket_dyn_templates::Template;
use std::fs;

use rand::distributions::{Alphanumeric, Standard, Uniform};
use rand::{thread_rng, Rng};

use chrono::prelude::*;

use crate::sorted_list::SortedList;
use crate::util::*;

#[derive(FromForm)]
struct CategoriseForm<'r> {
    category: &'r str,
    scan_id: &'r str,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Scan {
    id: uuid::Uuid,
    date: DateTime<Local>,
    items: Vec<ScanItem>,
}

impl Scan {
    fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            date: Local::now(),
            items: Vec::new(),
        }
    }

    fn new_with(count: usize) -> Self {
        let mut rng = thread_rng();
        let items = (0..count)
            .map(|_| {
                ScanItem::new::<String>(
                    // String:
                    (&mut rng)
                        .sample_iter(Alphanumeric)
                        .take(7)
                        .map(char::from)
                        .collect(),
                    rng.gen(),
                )
            })
            .collect();
        Self {
            id: uuid::Uuid::new_v4(),
            date: Local::now(),
            items,
        }
    }

    fn sorted_list() -> SortedList<Scan> {
        SortedList::new_on_field(|x: &Scan| x.date)
    }

    fn to_categorise<'a>(&'a self) -> Vec<&'a ScanItem> {
        self.items
            .iter()
            .filter(|x| x.needs_categorised())
            .collect()
    }

    fn count_done(&self) -> (usize, usize) {
        let done = self.items.iter().filter(|x| !x.needs_categorised()).count();
        (done, self.items.len())
    }

    fn categorise(&mut self, uuid: &str, category: &str) {
        let uuid = uuid::Uuid::parse_str(uuid).unwrap();
        if let Some(item) = self.items.iter_mut().filter(|x| x.id == uuid).next() {
            item.categorise(category);
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ScanItem {
    id: uuid::Uuid,
    name: String,
    price: f32,
    category: Option<String>,
}

impl ScanItem {
    fn new<S: Into<String>>(name: S, price: f32) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            price,
            category: None,
        }
    }

    fn needs_categorised(&self) -> bool {
        self.category.is_none()
    }

    fn categorise(&mut self, category: &str) {
        self.category = Some(category.to_string());
    }
}

struct ScanConfig {
    scans: Vec<Scan>,
    location: String,
}

type ScanConfigState = Arc<Mutex<ScanConfig>>;
impl Default for ScanConfig {
    fn default() -> Self {
        ScanConfig {
            scans: Vec::new(),
            location: String::from("Desk.toml"),
        }
    }
}

impl ScanConfig {
    fn new(path: &str) -> Self {
        Self {
            scans: vec![Scan::new_with(1), Scan::new_with(5)],
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
    "scan_config.json".to_string()
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
            scans,
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
fn get(scans: &State<ScanConfigState>) -> Template {
    let mut scans = get_mutexed(scans);

    let context = json!({
        "errors": [],
        "scans": scans.scans,
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
fn get_one(uuid: &str, scans: &State<ScanConfigState>) -> Template {
    let mut scans = get_mutexed(scans);
    let uuid = uuid::Uuid::parse_str(uuid).unwrap();

    let context = json!({
        "errors": [],
        "scan": scans.scans.iter().filter(|x| x.id == uuid).next(),
    });

    Template::render("scan/one", &context)
}

#[post("/<uuid>", data = "<user_input>")]
fn post_one(
    uuid: &str,
    user_input: Form<CategoriseForm<'_>>,
    scans: &State<ScanConfigState>,
) -> Redirect {
    let mut scans = get_mutexed(scans);
    let uuid = uuid::Uuid::parse_str(uuid).unwrap();

    if let Some(scan) = scans.scans.iter_mut().filter(|x| x.id == uuid).next() {
        scan.categorise(user_input.scan_id, user_input.category);
    }

    Redirect::to(format!("/scan/{}", uuid))
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .manage(Arc::new(Mutex::new(ScanConfig::default())))
        .mount("/scan", routes![get, new_get, new_post, get_one, post_one])
        .attach(AdHoc::on_liftoff("configure desk", configure_desk))
        .attach(AdHoc::config::<ScanConfigConfig>())
}
