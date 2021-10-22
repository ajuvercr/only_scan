use std::cmp::Ordering;


use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::response::Redirect;
use rocket::serde::json::serde_json::{self, json};
use rocket::serde::{Deserialize, Serialize};
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::net::TcpStream;
use rocket::{Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::repository::Repository;
use crate::util::*;

#[derive(Deserialize, Debug)]
struct DeskConfigConfig {
    #[serde(default = "default_location")]
    desk_config_location: String,
    #[serde(default = "default_desk_server_ip")]
    desk_server_ip: String,
    #[serde(default = "default_desk_server_port")]
    desk_server_port: u16,
}

fn default_location() -> String {
    "desk_config.json".to_string()
}

fn default_desk_server_ip() -> String {
    "127.0.0.1".into()
}

fn default_desk_server_port() -> u16 {
    9123
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
struct DeskStand {
    id: uuid::Uuid,
    name: String,
    amount: i32,
}

impl DeskStand {
    fn new(name: &str, amount: i32) -> Self {
        Self {
            name: name.into(),
            amount,
            id: uuid::Uuid::new_v4(),
        }
    }
}

impl PartialOrd for DeskStand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DeskStand {
    fn cmp(&self, other: &Self) -> Ordering {
        self.amount.cmp(&other.amount)
    }
}
type Desks = Repository<Vec<DeskStand>>;

async fn exec_command<T: Serialize>(command: T, config: &DeskConfigConfig) -> Option<String> {
    let mut stream = TcpStream::connect((config.desk_server_ip.as_str(), config.desk_server_port))
        .await
        .ok()?;

    let mut bytes = serde_json::to_vec(&command).ok()?;
    bytes.push(b'\n');
    stream.write_all(&bytes).await.ok()?;
    stream.flush().await.ok()?;

    let mut out = String::new();
    stream.read_to_string(&mut out).await.ok()?;

    Some(out)
}

#[get("/")]
fn get(desks: &State<Desks>) -> Template {
    desks
        .with( |desks| {
            let context = json!({
                "desks": desks,
                "errors": []
            });
            Template::render("desk", &context)
        })
}

#[derive(FromForm)]
struct NewDesk {
    name: String,
    amount: Option<i32>,
}

async fn try_get_current_height(config: &DeskConfigConfig) -> Option<i32> {
    let out = exec_command(json!({}), config).await?;
    out.lines()
        .filter(|x| x.starts_with("Height:"))
        .map(|x| x.trim_matches(|c: char| !c.is_ascii_digit()))
        .filter_map(|x| x.parse::<i32>().ok())
        .next()
}

#[post("/new", data = "<input>")]
async fn new_desk(
    input: Form<NewDesk>,
    desks: &State<Desks>,
    config: &State<DeskConfigConfig>,
) -> Result<Redirect, Template> {
    let amount = if let Some(amount) = input.amount {
        amount.into()
    } else {
        try_get_current_height(config.inner()).await
    };

    desks.with_save(
        |d| {
            match amount {
                Some(amount) => {
                    d.push(DeskStand::new(&input.name, amount));
                    d.sort();
                    Ok(Redirect::to("/desk"))
                }
                _ => {
                    let context = json!({
                        "desks": d,
                        "errors": [Error::new("Something failed", "Could not determine current height, please provide a value.")]
                    });

                    Err(Template::render("desk", &context))
                }
            }
        }
    )
}

#[post("/<uuid>")]
async fn post(
    uuid: &str,
    desks: &State<Desks>,
    config: &State<DeskConfigConfig>,
) -> Option<()> {
    #[derive(Serialize)]
    struct DeskAction {
        move_to: bool,
        move_to_raw: i32,
    }

    let optional_desk = desks.with(|d| {
        let uuid = uuid::Uuid::parse_str(uuid).unwrap();
        d.iter().find(|x| x.id == uuid).cloned()
    });

    if let Some(desk) = optional_desk {
        let action = DeskAction {
            move_to: true,
            move_to_raw: desk.amount,
        };

        exec_command(action, config.inner()).await?;
    }

    Some(())
}

#[get("/<uuid>/delete")]
fn delete(uuid: &str, desks: &State<Desks>) -> Redirect {
    desks.with_save(|desks| {
        let uuid = uuid::Uuid::parse_str(uuid).unwrap();
        desks.retain(|x| x.id != uuid);

        Redirect::to("/desk")
    })
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount("/desk", routes![get, post, new_desk, delete])
        .attach(AdHoc::config::<DeskConfigConfig>())
        .attach(Repository::<Vec<DeskStand>>::adhoc(
            "desk config",
            |c: &DeskConfigConfig| c.desk_config_location.to_string(),
            Vec::new(),
        ))
}
