use rocket::{fairing::AdHoc, response::Redirect, routes, Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::{
    context::Context,
    oauth::AuthUser,
};
use rocket::serde::Deserialize;

mod ingest;

#[derive(Deserialize, Debug)]
struct FavaConfig {
    #[serde(default = "fava_base")]
    fava_base: String,
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
    unwrap!(user);
    context.add("fava_base", config.fava_base.clone());
    Ok(Template::render("fava/fava", context.value()))
}

#[get("/")]
async fn index(context: Context) -> Result<Template, Redirect> {
    Ok(Template::render("fava/index", context.value()))
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    let rocket = ingest::fuel(rocket);
    rocket
        .mount("/fava", routes![index, beancount])
        .attach(AdHoc::config::<FavaConfig>())
}
