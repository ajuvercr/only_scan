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
    name: &'r str,
    price: f32,
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

    fn is_done(&self) -> bool {
        !self.items.iter().any(|x| x.needs_categorised())
    }

    fn get_first<'a>(&'a self) -> Option<&'a ScanItem> {
        self.items.iter().filter(|x| x.needs_categorised()).next()
    }

    fn count_done(&self) -> (usize, usize) {
        let done = self.items.iter().filter(|x| !x.needs_categorised()).count();
        (done, self.items.len())
    }

    fn categorise(&mut self, uuid: &str, name: &str, price: f32, category: &str) {
        let uuid = uuid::Uuid::parse_str(uuid).unwrap();
        if let Some(item) = self.items.iter_mut().filter(|x| x.id == uuid).next() {
            println!("got some scan item thing");
            item.category = Some(category.to_string());
            item.name = name.to_string();
            item.price = price;
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

    let scans: Vec<_> = scans.scans.iter().map(
        |scan| {
            json!({
                "date": scan.date.format("%d/%m/%C").to_string(),
                "done": scan.count_done().0,
                "total": scan.count_done().1,
                "id": scan.id,
            })
        }).collect();

    let context = json!({
        "errors": [],
        "scans": scans,
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

macro_rules! get_foo {
    (item mut $state:expr, $item_id:expr) => {
        $state
            .items
            .iter_mut()
            .filter(|x| x.id == $item_id)
            .next()?
    };
    (item $state:expr, $item_id:expr) => {
        $state.items.iter().filter(|x| x.id == $item_id).next()?
    };
    (scan mut $state:expr, $scan_id:expr) => {
        $state
            .scans
            .iter_mut()
            .filter(|x| x.id == $scan_id)
            .next()?
    };
    (scan $state:expr, $scan_id:expr) => {
        $state.scans.iter().filter(|x| x.id == $scan_id).next()?
    };
    (state $state:expr) => {
        get_mutexed($state)
    };
}

#[get("/<uuid_str>")]
fn get_scan(uuid_str: &str, scans: &State<ScanConfigState>) -> Option<Result<Template, Redirect>> {
    let uuid = uuid::Uuid::parse_str(uuid_str).unwrap();
    let state = get_foo!(state scans);
    let scan = get_foo!(scan state, uuid);

    if let Some(item) = scan.get_first() {
        Err(Redirect::to(uri!("/scan", get_one(uuid_str, item.id.to_string())))).into()
    } else {
        let context = json!({
            "errors": [],
        });

        Ok(Template::render("scan/one", &context)).into()
    }
}

#[get("/<scan_id_str>/<item_id_str>")]
fn get_one(
    scan_id_str: &str,
    item_id_str: &str,
    scans: &State<ScanConfigState>,
) -> Option<Template> {
    let scan_id = uuid::Uuid::parse_str(scan_id_str).ok()?;
    let item_id = uuid::Uuid::parse_str(item_id_str).ok()?;
    let state = get_foo!(state scans);
    let scan = get_foo!(scan state, scan_id);
    let item = get_foo!(item scan, item_id);

    let context = json!({
        "errors": [],
        "item": {
            "name": item.name,
            "price": item.price,
        }
    });

    Template::render("scan/item", &context).into()
}

#[post("/<scan_id_str>/<item_id_str>", data = "<user_input>")]
fn post_one(
    scan_id_str: &str,
    item_id_str: &str,
    user_input: Form<CategoriseForm<'_>>,
    scans: &State<ScanConfigState>,
) -> Redirect {
    let scan_id = uuid::Uuid::parse_str(scan_id_str).unwrap();
    // let item_id = uuid::Uuid::parse_str(item_id_str).unwrap();

    let mut scans = get_mutexed(scans);

    if let Some(scan) = scans.scans.iter_mut().filter(|x| x.id == scan_id).next() {
        println!("Got some scan");
        scan.categorise(
            item_id_str,
            user_input.name,
            user_input.price,
            user_input.category,
        );
    }

    Redirect::to(format!("/scan/{}", scan_id_str))
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .manage(Arc::new(Mutex::new(ScanConfig::default())))
        .mount(
            "/scan",
            routes![get, new_get, new_post, get_scan, get_one, post_one],
        )
        .attach(AdHoc::on_liftoff("configure desk", configure_desk))
        .attach(AdHoc::config::<ScanConfigConfig>())
}
