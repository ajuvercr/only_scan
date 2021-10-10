use std::borrow::Borrow;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::futures::future::BoxFuture;
use rocket::response::Redirect;
use rocket::serde::json::serde_json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{Build, Orbit, Request, Response, Rocket, State};
use rocket_dyn_templates::Template;
use std::fs;

use super::sorted_list::SortedList;

fn get_mutexed_rocket<'a, T>(rocket: &'a Rocket<Orbit>) -> impl DerefMut<Target = T> + 'a
where
    T: Sync + Send + 'static,
{
    rocket
        .state::<Arc<Mutex<T>>>()
        .expect("No state found!")
        .lock()
        .expect("Failed unlock rocket state")
}

fn get_mutexed<'a, T>(state: &'a State<Arc<Mutex<T>>>) -> impl DerefMut<Target = T> + 'a
where
    T: Sync + Send + 'static,
{
    state.inner().lock().expect("Failed unlock rocket state")
}

#[derive(Deserialize, Debug)]
struct DeskConfigConfig {
    #[serde(default = "default_location")]
    config_location: String,
}

fn default_location() -> String {
    "desk_config.json".to_string()
}

#[derive(Deserialize, Serialize, Debug)]
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
        println!("Rocket launch config: {:?}", rocket.config());

        if let Some(DeskConfigConfig { config_location }) = rocket.state::<DeskConfigConfig>() {
            let config = initial_read_state(config_location)
                .await
                .expect("Something failed");

            let mut desk_config = get_mutexed_rocket::<DeskConfig>(&rocket);
            *desk_config = config;
        }
    })
}

async fn read_file<T>(loc: &str) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(loc).ok()?;

    serde_json::from_str(&content).ok()
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
fn get(desks: &State<DeskConfigState>) -> Template {
    let d = get_mutexed(desks);

    let desks: &Vec<DeskStand> = d.stands.borrow();
    Template::render("desk", desks)
}

#[derive(FromForm)]
struct NewDesk {
    name: String,
    amount: i32,
}

#[post("/new", data = "<input>")]
fn new_desk(input: Form<NewDesk>, desks: &State<DeskConfigState>) -> Redirect {
    let mut d = get_mutexed(desks);
    d.stands.insert(DeskStand::new(&input.name, input.amount));
    d.save().unwrap();
    Redirect::to("/desk")
}

#[post("/<uuid>")]
fn post(uuid: &str, desks: &State<DeskConfigState>) -> () {
    let d = get_mutexed(desks);
    let uuid = uuid::Uuid::parse_str(uuid).unwrap();
    println!("Posting {}", uuid);

    if let Some(desk) = d.stands.iter().find(|x| x.id == uuid) {
        println!("Found {:?}", desk);
    }
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
