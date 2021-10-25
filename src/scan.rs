use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::response::Redirect;
use rocket::serde::json::serde_json::json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket::{Build, Rocket};
use rocket_dyn_templates::Template;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use chrono::prelude::*;
use regex::Regex;

use crate::repository::Repository;
use crate::util::{self, read_file};
use crate::vision;

const TEMP_FILE_1: &'static str = "/tmp/file1.jpg";
const TEMP_FILE_2: &'static str = "/tmp/file2.jpg";

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
        $state.iter().filter(|x| x.id == $scan_id).next()?
    };
    (state $state:expr) => {
        get_mutexed($state)
    };
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Beans {
    categories: Vec<(usize, String)>,
    pay_options: Vec<(usize, String)>,
}

impl Beans {
    fn new() -> Self {
        Self {
            categories: Vec::new(),
            pay_options: Vec::new(),
        }
    }

    fn inc_category(&mut self, category: &str) {
        self.categories
            .iter_mut()
            .filter(|(_, f)| f == category)
            .for_each(|(c, _)| {
                *c += 1;
            });
        self.categories.sort_unstable_by(|a, b| b.cmp(a));
    }

    fn inc_pay(&mut self, pay_option: &str) {
        self.pay_options
            .iter_mut()
            .filter(|(_, f)| f == pay_option)
            .for_each(|(c, _)| {
                *c += 1;
            });

        self.pay_options.sort_unstable_by(|a, b| b.cmp(a));
    }
}

#[derive(FromForm)]
struct CategoriseForm<'r> {
    category: &'r str,
    name: &'r str,
    price: usize,
}

type Scans = Repository<Vec<Scan>>;
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Scan {
    id: String,
    date: NaiveDate,
    items: Vec<ScanItem>,
}

impl Scan {

    fn delete(&mut self, id: &str) {
        self.items.retain(|x| x.id != id);
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
            id: uuid::Uuid::new_v4().to_string(),
            date: Local::today().naive_local(),
            items,
        }
    }

    pub fn from_vec(vec: Vec<Vec<String>>) -> Self {
        let str: String = vec
            .iter()
            .flatten()
            .cloned()
            .collect::<String>()
            .replace(" ", "");

        let re = Regex::new(r"(\d{2}/\d{2}/\d{2})").unwrap();
        let date: NaiveDate = re
            .captures(&str)
            .and_then(|caps| caps.get(0))
            .and_then(|x| NaiveDate::parse_from_str(x.as_str(), "%d/%m/%y").ok())
            .unwrap_or(Local::today().naive_local());

        let items = vec.into_iter().filter_map(ScanItem::try_from_vec).collect();

        let out = Self {
            id: uuid::Uuid::new_v4().to_string(),
            date,
            items,
        };

        out
    }

    fn get_first<'a>(&'a self) -> Option<&'a ScanItem> {
        self.items.iter().filter(|x| x.needs_categorised()).next()
    }

    fn count_done(&self) -> (usize, usize) {
        let done = self.items.iter().filter(|x| !x.needs_categorised()).count();
        (done, self.items.len())
    }

    fn categorise(&mut self, uuid: &str, name: &str, price: usize, category: &str) {
        if let Some(item) = self.items.iter_mut().filter(|x| x.id == uuid).next() {
            item.category = Some(category.to_string());
            item.name = name.to_string();
            item.price = price;
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ScanItem {
    id: String,
    name: String,
    price: usize,
    category: Option<String>,
}

impl ScanItem {
    fn new<S: Into<String>>(name: S, price: usize) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            price,
            category: None,
        }
    }

    fn needs_categorised(&self) -> bool {
        self.category.is_none()
    }

    fn try_from_vec(mut vec: Vec<String>) -> Option<Self> {
        if vec.len() < 2 {
            return None;
        }
        vec.reverse();
        let mut iter = vec.into_iter();

        let mut price = 0;
        let mut price_str = String::new();
        loop {
            price_str += &iter.next()?;

            if !price_str.contains('.') && !price_str.contains(',') {
                continue;
            }

            let price_try = price_str.replace(',', ".").parse::<f32>().ok();

            if let Some(p) = price_try {
                price = (p * 100.0) as usize;
                break;
            }
        }

        let name = iter.collect::<Vec<_>>().join(" ");
        if name.contains("\n") {
            return None;
        }

        Some(Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            price,
            category: None,
        })
    }
}

#[derive(Deserialize, Debug)]
struct ScanConfigConfig {
    #[serde(default = "default_location")]
    scan_config_location: String,
    #[serde(default = "default_beans_location")]
    bean_config_location: String,
}

fn default_location() -> String {
    "scan_config.json".to_string()
}

fn default_beans_location() -> String {
    "beans_cofig.json".to_string()
}

#[get("/")]
fn get(scans: &State<Scans>) -> Template {
    scans.with(|scans| {
        let scans: Vec<_> = scans
            .iter()
            .map(|scan| {
                json!({
                    "date": scan.date.format("%d/%m/%C").to_string(),
                    "done": scan.count_done().0,
                    "total": scan.count_done().1,
                    "id": scan.id,
                })
            })
            .collect();

        let context = json!({
            "errors": [],
            "scans": scans,
        });

        Template::render("scan/index", &context)
    })
}

#[get("/new")]
fn new_get() -> Template {
    let context = json!({
        "errors": []
    });

    Template::render("scan/new", &context)
}

#[derive(FromForm, Debug)]
struct NewScan {
    file: String,
}

#[post("/new", data = "<file>")]
async fn new_post(mut file: TempFile<'_>, scans: &State<Scans>) -> Option<Redirect> {
    file.persist_to(TEMP_FILE_1).await.ok()?;
    util::turn_image(TEMP_FILE_1, TEMP_FILE_2)?;

    let file = util::ocr(TEMP_FILE_2)?;
    let part = file.responses.into_iter().next()?;
    let lines = part.lines();

    let scan = Scan::from_vec(lines);
    let id = scan.id.clone();

    scans.with_save(|scans| scans.push(scan));

    Redirect::to(uri!("/scan", get_scan(id))).into()
}

#[get("/<uuid>")]
fn get_scan(
    uuid: &str,
    scans: &State<Scans>,
    beans: &State<Repository<Beans>>,
) -> Option<Result<Template, Redirect>> {
    scans.with(|state| {
        let scan = get_foo!(scan state, uuid);

        if let Some(item) = scan.get_first() {
            Err(Redirect::to(uri!(
                "/scan",
                get_one(uuid, item.id.to_string())
            )))
            .into()
        } else {
            beans.with(|beans| {
                let pay_options: Vec<_> = beans.pay_options.iter().map(|(_, x)| x).collect();
                let context = json!({
                    "errors": [],
                    "pay_options": pay_options,
                });

                Ok(Template::render("scan/one", &context)).into()
            })
        }
    })
}

#[get("/<scan_id>/<item_id>")]
fn get_one(
    scan_id: &str,
    item_id: &str,
    scans: &State<Scans>,
    beans: &State<Repository<Beans>>,
) -> Option<Template> {
    scans.with(|state| {
        let scan = get_foo!(scan state, scan_id);
        let item = get_foo!(item scan, item_id);

        beans.with(|beans| {
            let categories: Vec<_> = beans.categories.iter().map(|(_, x)| x).collect();

            let context = json!({
                "errors": [],
                "item": {
                    "name": item.name,
                    "price": item.price,
                },
                "categories": categories,
            });

            Template::render("scan/item", &context).into()
        })
    })
}

#[post("/<scan_id>/<item_id>", data = "<user_input>")]
fn post_one(
    scan_id: &str,
    item_id: &str,
    user_input: Form<CategoriseForm<'_>>,
    scans: &State<Scans>,
    beans: &State<Repository<Beans>>,
) -> Redirect {
    beans.with_save(|beans| {
        beans.inc_category(user_input.category);
    });

    scans.with_save(|scans| {
        if let Some(scan) = scans.iter_mut().filter(|x| x.id == scan_id).next() {
            scan.categorise(
                item_id,
                user_input.name,
                user_input.price,
                user_input.category,
            );
        }

        Redirect::to(format!("/scan/{}", scan_id))
    })
}

#[delete("/<scan_id>/<item_id>")]
fn delete_one(scan_id: &str, item_id: &str, scans: &State<Scans>) -> Redirect {
    println!("Deleting");
    scans.with_save(|scans| {
        if let Some(scan) = scans.iter_mut().filter(|x| x.id == scan_id).next() {
            scan.delete(item_id);
        }
    });

    Redirect::to(format!("/scan/{}", scan_id))
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount(
            "/scan",
            routes![get, new_get, new_post, get_scan, get_one, post_one, delete_one],
        )
        .attach(AdHoc::config::<ScanConfigConfig>())
        .attach(Repository::<Vec<Scan>>::adhoc(
            "scans config",
            |c: &ScanConfigConfig| c.scan_config_location.to_string(),
            vec![Scan::new_with(2), Scan::new_with(5)],
        ))
        .attach(Repository::<Beans>::adhoc(
            "beans config",
            |c: &ScanConfigConfig| c.bean_config_location.to_string(),
            Beans {
                categories: Vec::new(),
                pay_options: Vec::new(),
            },
        ))
}
