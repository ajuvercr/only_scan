use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::response::Redirect;
use rocket::serde::json::serde_json::json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket::{Build, Rocket};
use rocket_dyn_templates::Template;
use std::io::Write;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use regex::Regex;

use crate::repository::Repository;
mod vision;

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
    price: f32,
}

type Scans = Repository<Vec<Scan>>;
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Scan {
    id: String,
    date: time::Date,
    items: Vec<ScanItem>,
}

#[allow(deprecated)]
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
                    rng.gen_range(50..1000),
                )
            })
            .collect();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            date: time::Date::today(),
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
        let mut date: time::Date = re
            .captures(&str)
            .and_then(|caps| caps.get(0))
            .and_then(|x| time::Date::parse(x.as_str(), "%d/%m/%y").ok())
            .unwrap_or(time::Date::today());

        if date.year() < 2000 {
            date = time::Date::try_from_ymd(date.year() + 2000, date.month(), date.day()).unwrap();
        }

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

        let mut price_str = String::new();
        let price = loop {
            price_str += &iter.next()?;

            if !price_str.contains('.') && !price_str.contains(',') {
                continue;
            }

            let price_try = price_str.replace(',', ".").parse::<f32>().ok();

            if let Some(p) = price_try {
                break (p * 100.0) as usize;
            }
        };

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
    #[serde(default = "default_beancount_location")]
    beancount_location: String,
}

fn default_location() -> String {
    "scan_config.json".to_string()
}

fn default_beans_location() -> String {
    "beans_cofig.json".to_string()
}

fn default_beancount_location() -> String {
    "main.bean".to_string()
}

#[get("/")]
fn get(scans: &State<Scans>) -> Template {
    scans.with(|scans| {
        let scans: Vec<_> = scans
            .iter()
            .map(|scan| {
                json!({
                    "date": scan.date.format("%d/%m/%y").to_string(),
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

#[post("/new", data = "<file>")]
async fn new_post(mut file: TempFile<'_>, scans: &State<Scans>) -> Option<Redirect> {
    file.persist_to(TEMP_FILE_1).await.ok()?;
    vision::turn_image(TEMP_FILE_1, TEMP_FILE_2)?;

    let file = vision::ocr(TEMP_FILE_2)?;
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
                let categories: Vec<_> = beans.categories.iter().map(|(_, x)| x).collect();
                let total: usize = scan.items.iter().map(|item| item.price).sum();

                let items: Vec<_> = scan
                    .items
                    .iter()
                    .map(|item| {
                        json!({
                            "name": item.name,
                            "price": item.price,
                            "category": item.category,
                        })
                    })
                    .collect();
                let context = json!({
                    "errors": [],
                    "pay_options": pay_options,
                    "categories": categories,
                    "total": total,
                    "items": items,
                    "date": scan.date.format("%Y-%m-%d"),
                });

                Ok(Template::render("scan/one", &context)).into()
            })
        }
    })
}

#[derive(FromForm, Debug, Clone)]
struct FinishScan<'r> {
    name: &'r str,
    total: Vec<f64>,
    rest: &'r str,
    pay: Vec<&'r str>,
    date: time::Date,
}

#[derive(FromForm, Debug, Clone)]
struct Payment<'r> {
    total: usize,
    pay: &'r str,
}

use std::collections::HashMap;
struct ScanOutput<'r, 'b> {
    items: HashMap<&'r str, usize>,
    date: time::Date,
    name: &'r str,
    rest: &'r str,
    payments: &'b Vec<Payment<'r>>,
}

impl<'r, 'b> ScanOutput<'r, 'b> {
    pub fn new<T>(
        raw_items: T,
        date: time::Date,
        name: &'r str,
        rest: &'r str,
        payments: &'b Vec<Payment<'r>>,
    ) -> Self
    where
        T: IntoIterator<Item = &'b ScanItem>,
        'b: 'r,
    {
        let mut items = HashMap::new();

        for i in raw_items.into_iter().filter(|i| i.category.is_some()) {
            let cat = i.category.as_ref().unwrap().as_str();
            if let Some(vec) = items.get_mut(cat) {
                *vec += i.price;
            } else {
                items.insert(cat, i.price);
            }
        }

        Self {
            items,
            date,
            name,
            rest,
            payments,
        }
    }
}

use std::{fmt, fs};
impl fmt::Display for ScanOutput<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date_str = self.date.format("%Y-%m-%d");
        writeln!(f, "{} * \"{}\"", date_str, self.name)?;
        for payment in self.payments.iter() {
            writeln!(
                f,
                "    {} -{:.2}",
                payment.pay,
                payment.total as f64 / 100.0
            )?;
        }

        for (k, v) in self.items.iter() {
            writeln!(f, "    {} {:.2}", k, *v as f64 / 100.0)?;
        }

        if self.items.values().sum::<usize>()
            != self.payments.iter().map(|p| p.total).sum::<usize>()
        {
            writeln!(f, "    {}", self.rest)?;
        }

        Ok(())
    }
}

#[post("/<scan_id>", data = "<user_input>")]
fn post_scan(
    scan_id: &str,
    user_input: Form<FinishScan<'_>>,
    scans: &State<Scans>,
    beans: &State<Repository<Beans>>,
    config: &State<ScanConfigConfig>,
) -> Option<Redirect> {
    println!("{:?}", user_input);

    let location = &config.beancount_location;
    let zips = user_input.pay.iter().zip(user_input.total.iter());
    let payments: Vec<_> = zips
        .map(|(pay, &total)| Payment {
            total: (total * 100.0) as usize,
            pay,
        })
        .collect();

    scans.with_save(|scans| {
        let scan_index = scans.iter().position(|x| x.id == scan_id)?;
        let scan = scans.get_mut(scan_index)?;

        let output = ScanOutput::new(
            &scan.items,
            user_input.date,
            user_input.name,
            user_input.rest,
            &payments,
        );

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(location)
            .ok()?;

        writeln!(file, "\n{}", output).ok()?;

        beans.with_save(|beans| {
            for pay in user_input.pay.iter() {
                beans.inc_pay(pay);
            }
        });

        scans.remove(scan_index);
        Redirect::to("/scan").into()
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
            let (left, right) = beans.categories.split_at(9);

            // let categories: Vec<_> = beans.categories.iter().map(|(_, x)| x).collect();

            let context = json!({
                "errors": [],
                "item": {
                    "name": item.name,
                    "price": item.price,
                },
                "categories_left": left,
                "categories_right": right,
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
                (user_input.price * 100.0) as usize,
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
            routes![get, new_get, new_post, get_scan, post_scan, get_one, post_one, delete_one],
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
