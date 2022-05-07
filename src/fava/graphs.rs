use rocket::{response::Redirect, Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::{context::Context, fava::ScanConfigConfig, oauth::AuthUser, util::read_command};

#[get("/")]
fn index(mut context: Context, user: AuthUser) -> Result<Template, Redirect> {
    unwrap!(user);
    Ok(Template::render("fava/graphs", context.value()))
}

#[get("/input.csv")]
fn input(user: AuthUser, config: &State<ScanConfigConfig>) -> Result<String, Redirect> {
    unwrap!(user);

    let location = &config.beancount_location;

    // bean-query -f csv main.bean 'select date, flag, account, number'
    let out = read_command(vec![
        "bean-query",
        "-f",
        "csv",
        location,
        "select date, flag, account, number",
    ])
    .unwrap_or_default();

    Ok(out)
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount("/fava/graphs", routes![index, input])
}
