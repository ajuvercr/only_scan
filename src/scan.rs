use std::borrow::Borrow;
use std::ops::{Deref, DerefMut};
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
struct ScanConfigConfig {
    #[serde(default = "default_location")]
    scan_config_location: String,
}

fn default_location() -> String {
    "desk_config.json".to_string()
}


fn configure_desk<'a>(rocket: &'a Rocket<Orbit>) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        println!("Rocket launch config: {:?}", rocket.config());

        if let Some(DeskConfigConfig {
            config_location, ..
        }) = rocket.state::<DeskConfigConfig>()
        {
            let config = initial_read_state(config_location)
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
    rocket.mount("/scan", routes![get, new_get, new_post, get_one, post_one])
}
