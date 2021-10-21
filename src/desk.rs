use std::borrow::Borrow;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::futures::future::BoxFuture;
use rocket::response::Redirect;
// use rocket::serde::json::json;
use rocket::serde::json::serde_json::{self, json};
use rocket::serde::{Deserialize, Serialize};
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::net::TcpStream;
use rocket::{Build, Orbit, Rocket, State};
use rocket_dyn_templates::Template;
use std::fs;

use crate::sorted_list::SortedList;
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

#[derive(Deserialize, Serialize, Debug, Clone)]
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

    fn sorted_list() -> SortedList<DeskStand> {
        SortedList::new_on_field(|x: &DeskStand| -x.amount)
    }
}

#[derive(Debug)]
struct DeskConfig {
    stands: SortedList<DeskStand>,
    location: String,
}

type DeskConfigState = Arc<Mutex<DeskConfig>>;
impl Default for DeskConfig {
    fn default() -> Self {
        DeskConfig {
            stands: DeskStand::sorted_list(),
            location: String::from("Desk.toml"),
        }
    }
}

impl DeskConfig {
    fn new(path: &str) -> Self {
        Self {
            stands: DeskStand::sorted_list(),
            location: path.to_string(),
        }
    }

    fn save(&self) -> Option<()> {
        let vec = serde_json::to_vec_pretty(&self.stands.deref()).ok()?;
        fs::write(&self.location, &vec).ok()
    }
}

fn configure_desk<'a>(rocket: &'a Rocket<Orbit>) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        if let Some(DeskConfigConfig {
            desk_config_location,
            ..
        }) = rocket.state::<DeskConfigConfig>()
        {
            let config = initial_read_state(desk_config_location)
                .await
                .expect("Something failed");

            let mut desk_config = get_mutexed_rocket::<DeskConfig>(&rocket);
            *desk_config = config;
        }
    })
}

async fn initial_read_state(location: &str) -> Option<DeskConfig> {
    match read_file::<Vec<DeskStand>>(location).await {
        Some(stands) => Some(DeskConfig {
            stands: DeskStand::sorted_list().with_inner(stands),
            location: location.into(),
        }),
        None => {
            let mut out = DeskConfig::new(location);

            out.stands.insert(DeskStand::new("epic place", 401));

            out.save()?;
            Some(out)
        }
    }
}

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
fn get(desks: &State<DeskConfigState>) -> Template {
    let d = get_mutexed(desks);

    let desks: &Vec<DeskStand> = d.stands.borrow();

    let context = json!({
        "desks": desks,
        "errors": []
    });

    Template::render("desk", &context)
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
    desks: &State<DeskConfigState>,
    config: &State<DeskConfigConfig>,
) -> Result<Redirect, Template> {
    let amount = if let Some(amount) = input.amount {
        amount.into()
    } else {
        try_get_current_height(config.inner()).await
    };

    let mut d = get_mutexed(desks);
    match amount {
        Some(amount) => {
            d.stands.insert(DeskStand::new(&input.name, amount));
            d.save().unwrap();
            Ok(Redirect::to("/desk"))
        }
        _ => {
            let desks: &Vec<DeskStand> = d.stands.borrow();

            let context = json!({
                "desks": desks,
                "errors": [Error::new("Something failed", "Could not determine current height, please provide a value.")]
            });

            Err(Template::render("desk", &context))
        }
    }
}

#[post("/<uuid>")]
async fn post(
    uuid: &str,
    desks: &State<DeskConfigState>,
    config: &State<DeskConfigConfig>,
) -> Option<()> {
    #[derive(Serialize)]
    struct DeskAction {
        move_to: bool,
        move_to_raw: i32,
    }

    let optional_desk = {
        let d = get_mutexed(desks);
        let uuid = uuid::Uuid::parse_str(uuid).unwrap();
        d.stands.iter().find(|x| x.id == uuid).cloned()
    };

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
fn delete(uuid: &str, desks: &State<DeskConfigState>) -> Redirect {
    let mut desks = get_mutexed(desks);
    let uuid = uuid::Uuid::parse_str(uuid).unwrap();
    desks.stands.retain(|x| x.id != uuid);
    desks.save().unwrap();

    Redirect::to("/desk")
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .manage(Arc::new(Mutex::new(DeskConfig::default())))
        .mount("/desk", routes![get, post, new_desk, delete])
        .attach(AdHoc::on_liftoff("configure desk", configure_desk))
        .attach(AdHoc::config::<DeskConfigConfig>())
}
