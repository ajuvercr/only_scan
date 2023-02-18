use rocket::{fairing::AdHoc, response::Redirect, routes, Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::{context::Context, oauth::AuthUser};
use rocket::serde::Deserialize;

mod graphs;
mod ingest;
mod models;

#[derive(Deserialize, Debug)]
struct FavaConfig {
    #[serde(default = "fava_base")]
    fava_base: String,
}

#[derive(Deserialize, Debug)]
struct ScanConfigConfig {
    #[serde(default = "default_location")]
    ingest_file_location: String,
    #[serde(default = "default_beancount_location")]
    beancount_location: String,
}

fn default_location() -> String {
    "scan_config.json".to_string()
}

fn default_beancount_location() -> String {
    "main.bean".to_string()
}

fn fava_base() -> String {
    "https://avercruysse.be/fava/".to_string()
}

#[get("/beancount")]
async fn beancount(
    mut context: Context,
    user: AuthUser,
    config: &State<FavaConfig>,
) -> Result<Template, Redirect> {
    user.check()?;
    context.add("fava_base", config.fava_base.clone());
    Ok(Template::render("fava/fava", context.value()))
}

#[get("/")]
async fn index() -> Redirect {
    Redirect::to("/fava/ingest")
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    let rocket = ingest::fuel(rocket);
    let rocket = graphs::fuel(rocket);
    rocket
        .mount("/fava", routes![index, beancount])
        .attach(AdHoc::config::<FavaConfig>())
}
