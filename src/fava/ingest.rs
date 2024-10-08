use rocket::tokio::io::AsyncReadExt;
use rocket::Data;
use rocket::{fairing::AdHoc, response::Redirect, routes, Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::fava::ScanConfigConfig;
use crate::{context::Context, oauth::AuthUser};
use rocket::serde::json::serde_json::json;
use rocket::serde::Serialize;

use rocket::data::ToByteUnit;
use rocket::form::Form;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;

use crate::repository::Repository;

use super::models::*;

macro_rules! get_foo {
    (item mut $state:expr, $item_id:expr) => {
        $state
            .items
            .iter_mut()
            .filter(|x| x.id == $item_id)
            .next()?
    };
    (item $state:expr, $item_id:expr) => {
        $state.grouped.iter().filter(|x| x.key == $item_id).next()?
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

#[derive(Serialize, PartialEq, PartialOrd, Ord, Eq, Clone, Debug)]
struct Account {
    full: String,
    segments: Vec<(String, &'static str)>,
}

#[derive(Serialize, Debug, Clone)]
struct FavaAccounts {
    accounts: Vec<Account>,
    pay_options: Vec<Account>,
}

const COLORS: &[&'static str] = &[
    "#F5E1FF", "#FFC107", "#FF4081", "#00BCD4", "#F44336", "#9C27B0", "#8BC34A", "#FF9800",
    "#E91E63", "#4CAF50", "#2196F3", "#673AB7", "#FFEB3B", "#00FFBF", "#FF5722", "#607D8B",
    "#FFCDD2", "#CFD8DC", "#B0BEC5", "#FFD54F",
];

impl FavaAccounts {
    fn parse_account_line(
        line: &str,
        count: &mut usize,
        colors: &mut HashMap<String, &'static str>,
    ) -> Option<Account> {
        let mut words = line.split(" ");
        words.next(); // Date
        let action = words.next()?;
        if action != "open" {
            return None;
        }

        let full = words.next().map(String::from)?;
        let segments = full
            .split(':')
            .map(|x| {
                if let Some(color) = colors.get(x) {
                    (x.to_string(), *color)
                } else {
                    let color = COLORS[*count % COLORS.len()];
                    *count += 1;
                    colors.insert(x.to_string(), color);
                    (x.to_string(), color)
                }
            })
            .collect();

        Some(Account { full, segments })
    }

    fn parse_name_assets(x: &str) -> Option<&str> {
        let mut words = x.split(" ");
        let first = words.next()?;
        let scd = words.next()?;
        let third = words.next()?;
        if (first, scd) == ("option", "\"name_assets\"") {
            // Strip quotes
            Some(&third[1..third.len() - 1])
        } else {
            None
        }
    }

    pub fn init(config: &ScanConfigConfig) -> Self {
        let bean_file =
            fs::read_to_string(&config.beancount_location).expect("No beancount file found!");

        let mut count = 0;
        let mut colors = HashMap::new();

        let mut accounts: Vec<_> = bean_file
            .lines()
            .flat_map(|x| FavaAccounts::parse_account_line(x, &mut count, &mut colors))
            .collect();

        let name_assets = bean_file
            .lines()
            .find_map(FavaAccounts::parse_name_assets)
            .unwrap_or("Assets");

        accounts.sort();
        let pay_options: Vec<_> = accounts
            .iter()
            .filter(|x| x.full.starts_with(name_assets))
            .cloned()
            .collect();

        Self {
            accounts,
            pay_options,
        }
    }
}

#[derive(FromForm)]
struct CategoriseForm<'r> {
    category: &'r str,
}

#[get("/")]
fn get(scans: &State<Scans>, user: AuthUser, mut ctx: Context) -> Result<Template, Redirect> {
    user.check()?;
    scans.with(|scans| {
        let scans: Vec<_> = scans
            .iter()
            .map(|scan| {
                json!({
                    "done": scan.count_done().0,
                    "total": scan.count_done().1,
                    "id": scan.id,
                })
            })
            .collect();

        ctx.merge(json!({
        "scans": scans,

        }));
        Ok(Template::render("fava/ingest/index", &ctx.value()))
    })
}

#[post("/new", data = "<data>")]
async fn new_post(
    data: Data<'_>,
    scans: &State<Scans>,
    user: AuthUser,
) -> Result<Option<Redirect>, Redirect> {
    user.check()?;

    let mut buf = Vec::new();
    data.open(512u32.megabytes()).read_to_end(&mut buf).await
        .map_err(|e| {
            eprintln!("{:?}", e);
            Redirect::to("/fava")
        })?;
    
    let string = String::from_utf8_lossy(&buf);
        // .read_to_string(&mut string)
        // .await

    // let string = data
    //     .open(512u32.megabytes())
    //     .into_string()
    //     .await
    //     .map_err(|e| {
    //         eprintln!("{:?}", e);
    //         Redirect::to("/fava")
    //     })?
    //     .into_inner();

    let mut cursor = Cursor::new(string.replace(",", "."));

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(&mut cursor);

    let mut items = Vec::new();
    for result in rdr.deserialize() {
        let record: StatementUgly = result.map_err(|_| Redirect::to("/fava"))?;
        items.push(record.into());
    }
    let scan = Scan::new(items);

    scans.with_save(|r| r.push(scan));

    Ok(Some(Redirect::to("/fava")))
}

#[get("/<uuid>")]
fn get_scan(
    mut context: Context,
    uuid: &str,
    scans: &State<Scans>,
    accounts: &State<FavaAccounts>,
    user: AuthUser,
) -> Option<Result<Template, Redirect>> {
    user.check().ok()?;

    scans.with(|state| {
        let scan = get_foo!(scan state, uuid);

        if let Some(item) = scan.get_first() {
            Err(Redirect::to(uri!(
                "/fava/ingest",
                get_one(uuid, item.key.to_string())
            )))
            .into()
        } else {
            let per_category = scan.grouped.iter().flat_map(|x| x.category.as_ref()).fold(
                std::collections::HashMap::new(),
                |mut h, e| {
                    if let Some(c) = h.get_mut(e) {
                        *c += 1;
                    } else {
                        h.insert(e.clone(), 1);
                    }
                    h
                },
            );

            let total: usize = scan.grouped.iter().map(|x| x.statements.len()).sum();

            // TODO
            let add = json! {{
                "pay_options": accounts.pay_options,
                "total": total,
                "per_category": per_category,
            }};

            context.merge(add);

            Ok(Template::render("fava/ingest/last", context.value())).into()
        }
    })
}

#[derive(FromForm, Debug, Clone)]
struct Payment<'r> {
    pay: &'r str,
}

use std::io::Write;

#[post("/<scan_id>", data = "<user_input>")]
fn post_scan(
    scan_id: &str,
    user_input: Form<Payment<'_>>,
    scans: &State<Scans>,
    config: &State<ScanConfigConfig>,
    user: AuthUser,
) -> Option<Redirect> {
    if let Err(e) = user.check() {
        return Some(e);
    }

    let location = &config.beancount_location;

    scans.with_save(|scans| {
        let scan_index = scans.iter().position(|x| x.id == scan_id)?;
        let scan = scans.get_mut(scan_index)?;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(location)
            .ok()?;

        let mut items: Vec<_> = scan
            .grouped
            .iter()
            .flat_map(|x| x.statements.iter().cloned())
            .collect();

        items.sort_by_key(|x| x.date);
        for item in items.into_iter() {
            let output = item.to_output(user_input.pay);
            writeln!(file, "\n{}", output).ok()?;
        }

        scans.remove(scan_index);
        Redirect::to("/fava/ingest").into()
    })
}

#[get("/<scan_id>/<item_id>")]
fn get_one(
    scan_id: &str,
    item_id: &str,
    scans: &State<Scans>,
    accounts: &State<FavaAccounts>,
    mut context: Context,
    user: AuthUser,
) -> Option<Result<Template, Redirect>> {
    user.check().ok()?;

    scans.with(|state| {
        let scan = get_foo!(scan state, scan_id);
        let item = get_foo!(item scan, item_id);

        let total = item.total();
        let items = json!({
            "errors": [],
            "item": item,
            "accounts": accounts.accounts,
            "total": total
        });

        context.merge(items);
        Ok(Template::render("fava/ingest/item", context.value())).into()
    })
}

#[post("/<scan_id>/<item_id>", data = "<user_input>")]
fn post_one(
    scan_id: &str,
    item_id: &str,
    user_input: Form<CategoriseForm<'_>>,
    scans: &State<Scans>,
    user: AuthUser,
) -> Redirect {
    if let Err(e) = user.check() {
        return e;
    }

    scans.with_save(|scans| {
        if let Some(scan) = scans.iter_mut().filter(|x| x.id == scan_id).next() {
            scan.categorise(item_id, user_input.category);
        }

        Redirect::to(format!("/fava/ingest/{}", scan_id))
    })
}

#[delete("/<scan_id>/<item_id>")]
fn delete_group(scan_id: &str, item_id: &str, scans: &State<Scans>, user: AuthUser) -> Redirect {
    if let Err(e) = user.check() {
        return e;
    }

    println!("Deleting");
    scans.with_save(|scans| {
        if let Some(scan) = scans.iter_mut().filter(|x| x.id == scan_id).next() {
            scan.delete(item_id);
        }
    });

    Redirect::to(format!("/fava/ingest/{}", scan_id))
}

#[delete("/<scan_id>/<group_id>/<item_id>")]
fn delete_one(
    scan_id: &str,
    group_id: &str,
    item_id: &str,
    scans: &State<Scans>,
    user: AuthUser,
) -> Redirect {
    if let Err(e) = user.check() {
        return e;
    }

    println!("Deleting {} {} {}", scan_id, group_id, item_id);
    scans.with_save(|scans| {
        if let Some(scan) = scans.iter_mut().filter(|x| x.id == scan_id).next() {
            scan.delete_item(group_id, item_id);
        }
    });

    println!("Redirecting to /fava/ingest/{}", scan_id);
    Redirect::to(format!("/fava/ingest/{}", scan_id))
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount(
            "/fava/ingest",
            routes![
                get,
                new_post,
                get_scan,
                post_scan,
                get_one,
                post_one,
                delete_group,
                delete_one
            ],
        )
        .attach(AdHoc::config::<ScanConfigConfig>())
        .attach(AdHoc::try_on_ignite("beans", |rocket| {
            Box::pin(async move {
                if let Some(config) = rocket.state::<ScanConfigConfig>() {
                    let accounts = FavaAccounts::init(&config);
                    Ok(rocket.manage(accounts))
                } else {
                    Err(rocket)
                }
            })
        }))
        .attach(Repository::<Vec<Scan>>::adhoc(
            "scans config",
            |c: &ScanConfigConfig| c.ingest_file_location.to_string(),
            vec![],
        ))
}
