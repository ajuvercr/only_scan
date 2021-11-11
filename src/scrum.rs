use rocket::fairing::AdHoc;
use rocket::serde::{Deserialize, Serialize};
use rocket::{Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::repository::Repository as Repo;
use crate::util;

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Scrum {
    epics: Vec<Epic>,
    stories: Vec<Story>,
}

impl Scrum {
    pub fn nothing() -> Self {
        let epic = Epic::new(
            "epic epic",
            "In the comming years this epic should be handled successfully",
            Status::Doing,
        );
        Self {
            stories: vec![
                Story::new(
                    "small story",
                    "Some stories are small, others are big",
                    Status::Doing,
                    &epic,
                ),
                Story::new(
                    "to big story",
                    "Some stories are small, others are big",
                    Status::Todo,
                    &epic,
                ),
                Story::new(
                    "small story",
                    "Some stories are small, others are big",
                    Status::Done,
                    &epic,
                ),
                Story::new(
                    ":( story",
                    "Some stories are small, others are big",
                    Status::Doing,
                    None,
                ),
            ],
            epics: vec![epic],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
enum Status {
    Todo,
    Doing,
    Done,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Epic {
    id: String,
    title: String,
    content: String,
    status: Status,
}

impl Epic {
    pub fn new<S: Into<Option<Status>>>(title: &str, content: &str, status: S) -> Self {
        Self {
            id: util::id(),
            title: title.to_string(),
            content: content.to_string(),
            status: status.into().unwrap_or(Status::Todo),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Story {
    id: String,
    title: String,
    content: String,
    status: Status,
    today: bool,
    epic: Option<String>,
}

impl Story {
    pub fn new<'a, S: Into<Option<Status>>, E: Into<Option<&'a Epic>>>(
        title: &str,
        content: &str,
        status: S,
        epic: E,
    ) -> Self {
        Self {
            id: util::id(),
            title: title.to_string(),
            content: content.to_string(),
            status: status.into().unwrap_or(Status::Todo),
            today: false,
            epic: epic.into().map(|epic| epic.id.clone()),
        }
    }
}

#[derive(Deserialize, Debug)]
struct ScrumConfig {
    #[serde(default = "default_location")]
    scrum_location: String,
}
fn default_location() -> String {
    "scrum.json".to_string()
}

#[get("/someid")]
fn get_one() -> String {
    String::from("hallo")
}

#[get("/")]
fn get(scrum: &State<Repo<Scrum>>) -> Template {
    scrum.with(|scrum| {
        Template::render("scrum/index", scrum)
    })
    // String::from("hallo")
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount("/scrum", routes![get, get_one])
        .attach(AdHoc::config::<ScrumConfig>())
        .attach(Repo::<Scrum>::adhoc(
            "scrum",
            |c: &ScrumConfig| c.scrum_location.to_string(),
            Scrum::nothing(),
        ))
}
