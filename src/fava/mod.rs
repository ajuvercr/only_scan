use rocket::{fairing::AdHoc, response::Redirect, routes, Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::{
    context::Context,
    oauth::{AuthUser, User},
};
use rocket::serde::json::serde_json::{self, json};
use rocket::serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
struct FavaConfig {
    #[serde(default = "fava_base")]
    fava_base: String,
}

fn fava_base() -> String {
    "https://avercruysse.be/fava/".to_string()
}

#[get("/")]
async fn index(
    mut context: Context,
    user: AuthUser,
    config: &State<FavaConfig>,
) -> Result<Template, Redirect> {
    let user: User = unwrap!(user);
    context.add("fava_base", config.fava_base.clone());
    Ok(Template::render("fava/fava", context.value()))
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount("/fava", routes![index])
        .attach(AdHoc::config::<FavaConfig>())
}
