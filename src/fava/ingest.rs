use rocket::Data;
use rocket::{fairing::AdHoc, response::Redirect, routes, Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::fava::ScanConfigConfig;
use crate::{context::Context, oauth::AuthUser};
use rocket::serde::json::serde_json::json;
use rocket::serde::{Deserialize, Serialize};

use rocket::data::ToByteUnit;
use rocket::form::Form;
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
struct FavaAccount {
    short: String,
    name: String,
    children: Vec<FavaAccount>,
}
impl FavaAccount {
    fn sort(&mut self) {
        self.children.sort_by(|a, b| a.short.cmp(&b.short));
        self.children.iter_mut().for_each(FavaAccount::sort);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct FavaAccounts {
    accounts: Vec<FavaAccount>,
}

impl FavaAccounts {
    fn add_account(accounts: &mut FavaAccount, line: &str) -> Option<()> {
        let mut words = line.split(" ");
        words.next(); // Date
        let action = words.next()?;
        if action != "open" {
            return None;
        }

        let parts = words.next()?.split(":");
        let mut first = true;
        let mut current_total = "".to_string();

        parts.fold(accounts, |acc, part| {
            if !first {
                current_total += ":";
            } else {
                first = false;
            }

            current_total += part;

            if let Some(idx) = acc.children.iter().position(|x| x.short == part) {
                &mut acc.children[idx]
            } else {
                let n = FavaAccount {
                    short: part.to_string(),
                    name: current_total.clone(),
                    children: Vec::new(),
                };
                acc.children.push(n);
                acc.children.last_mut().unwrap()
            }
        });

        Some(())
    }

    pub async fn init(config: &ScanConfigConfig) -> Self {
        let bean_file =
            fs::read_to_string(&config.beancount_location).expect("No beancount file found!");

        let mut root = FavaAccount {
            short: "".into(),
            name: "".into(),
            children: Vec::new(),
        };

        for line in bean_file.lines() {
            FavaAccounts::add_account(&mut root, line);
        }

        root.sort();

        Self {
            accounts: root.children,
        }
    }
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
}

#[get("/")]
fn get(scans: &State<Scans>, user: AuthUser, mut ctx: Context) -> Result<Template, Redirect> {
    unwrap!(user);
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
        Ok(Template::render("fava/ingest", &ctx.value()))
    })
}

#[post("/new", data = "<data>")]
async fn new_post(
    data: Data<'_>,
    scans: &State<Scans>,
    user: AuthUser,
) -> Result<Option<Redirect>, Redirect> {
    unwrap!(user);

    let string = data
        .open(512u32.megabytes())
        .into_string()
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            Redirect::to("/fava")
        })?
        .into_inner();

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
    uuid: &str,
    scans: &State<Scans>,
    beans: &State<Repository<Beans>>,
    mut context: Context,
    user: AuthUser,
) -> Option<Result<Template, Redirect>> {
    unwrap!(user);

    scans.with(|state| {
        let scan = get_foo!(scan state, uuid);

        if let Some(item) = scan.get_first() {
            Err(Redirect::to(uri!(
                "/fava/ingest",
                get_one(uuid, item.id.0.to_string())
            )))
            .into()
        } else {
            beans.with(|beans| {
                let options = beans.pay_options.iter().map(|(_, x)| x).collect::<Vec<_>>();

                let per_category = scan.items.iter().flat_map(|x| x.category.as_ref()).fold(
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

                let add = json! {{
                    "pay_options": options,
                    "total": scan.items.len(),
                    "per_category": per_category,
                }};

                context.merge(add);

                Ok(Template::render("fava/ingest_last", context.value())).into()
            })
        }
    })
}

// #[derive(FromForm, Debug, Clone)]
// struct FinishScan;

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
    beans: &State<Repository<Beans>>,
    config: &State<ScanConfigConfig>,
    user: AuthUser,
) -> Option<Redirect> {
    if let Err(e) = user.r() {
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

        scan.items.sort_by_key(|x| x.date);
        for item in scan.items.iter() {
            let output = item.to_output(user_input.pay);
            writeln!(file, "\n{}", output).ok()?;
        }

        beans.with_save(|beans| {
            beans.inc_pay(user_input.pay);
        });

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
    beans: &State<Repository<Beans>>,
    mut context: Context,
    user: AuthUser,
) -> Option<Result<Template, Redirect>> {
    unwrap!(user);
    scans.with(|state| {
        let scan = get_foo!(scan state, scan_id);
        let item = get_foo!(item scan, ID(item_id.to_string()));

        beans.with(|beans| {
            // let (_left, right) = beans.categories.split_at(9);

            // let categories: Vec<_> = beans.categories.iter().map(|(_, x)| x).collect();

            let items = json!({
                "errors": [],
                "item": item,
                "categories_left": beans.categories,
                "accounts": accounts.accounts,
            });

            context.merge(items);
            Ok(Template::render("scan/item", context.value())).into()
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
    user: AuthUser,
) -> Redirect {
    if let Err(e) = user.r() {
        return e;
    }

    beans.with_save(|beans| {
        beans.inc_category(user_input.category);
    });

    scans.with_save(|scans| {
        if let Some(scan) = scans.iter_mut().filter(|x| x.id == scan_id).next() {
            scan.categorise(item_id, user_input.category);
        }

        Redirect::to(format!("/fava/ingest/{}", scan_id))
    })
}

#[delete("/<scan_id>/<item_id>")]
fn delete_one(scan_id: &str, item_id: &str, scans: &State<Scans>, user: AuthUser) -> Redirect {
    if let Err(e) = user.r() {
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

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount(
            "/fava/ingest",
            routes![get, new_post, get_scan, post_scan, get_one, post_one, delete_one],
        )
        .attach(AdHoc::config::<ScanConfigConfig>())
        .attach(AdHoc::try_on_ignite("beans", |rocket| {
            Box::pin(async move {
                if let Some(config) = rocket.state::<ScanConfigConfig>() {
                    let accounts = FavaAccounts::init(&config).await;
                    Ok(rocket.manage(accounts))
                } else {
                    Err(rocket)
                }
            })
        }))
        .attach(Repository::<Vec<Scan>>::adhoc(
            "scans config",
            |c: &ScanConfigConfig| c.scan_config_location.to_string(),
            vec![],
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
