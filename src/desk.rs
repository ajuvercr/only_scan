use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use rocket::fairing::AdHoc;
use rocket::futures::future::BoxFuture;
use rocket::serde::json::serde_json;
use rocket::serde::{Deserialize, Serialize};
use rocket::tokio::fs;
use rocket::{Build, Orbit, Rocket, State};
use rocket_dyn_templates::Template;

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

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct DeskConfig {
    stands: BTreeMap<u32, String>,
    location: String,
}
type DeskConfigState = Arc<Mutex<DeskConfig>>;

impl Default for DeskConfig {
    fn default() -> Self {
        DeskConfig {
            stands: BTreeMap::new(),
            location: String::from("Desk.toml"),
        }
    }
}

impl DeskConfig {
    fn new(path: &str) -> Self {
        Self {
            stands: BTreeMap::new(),
            location: path.to_string(),
        }
    }

    async fn save(&self) -> Option<()> {
        let vec = serde_json::to_vec_pretty(&self.stands).ok()?;
        fs::write(&self.location, &vec).await.ok()
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
    let content = fs::read_to_string(loc).await.ok()?;

    serde_json::from_str(&content).ok()
}

async fn initial_read_state(location: &str) -> Option<DeskConfig> {
    match read_file::<BTreeMap<u32, String>>(location).await {
        Some(stands) => Some(DeskConfig {
            stands,
            location: location.into(),
        }),
        None => {
            let mut out = DeskConfig::new(location);
            out.stands.insert(401, "epic place".into());

            out.save().await?;
            Some(out)
        }
    }
}

#[get("/")]
fn get(desks: &State<DeskConfigState>) -> Template {
    let d = get_mutexed(desks);

    let desks: Vec<_> = d.stands.iter().collect();

    // #[derive(Serialize)]
    // struct IndexContext {
    //     firstname: String,
    //     lastname: String,
    // }

    // let context = IndexContext {
    //     firstname: String::from("Arthur"),
    //     lastname: String::from("Meeee"),
    // };

    Template::render("index", &desks)
}

#[post("/<amount>")]
fn post(amount: u32) -> () {
    println!("Posting {}", amount);
}


pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .manage(Arc::new(Mutex::new(DeskConfig::default())))
        .mount("/desk", routes![get, post])
        .attach(AdHoc::on_liftoff("configure desk", configure_desk))
        .attach(AdHoc::config::<DeskConfigConfig>())
}
