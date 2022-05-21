use rocket::{fs::TempFile, response::Redirect, Build, Rocket, State};
use rocket_dyn_templates::Template;

use crate::{context::Context, fava::ScanConfigConfig, oauth::AuthUser, util::read_command};

#[get("/")]
fn index(context: Context, user: AuthUser) -> Result<Template, Redirect> {
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

#[derive(FromForm, Debug)]
struct Upload<'r> {
    file: TempFile<'r>,
}

use rocket::form::Form;
#[post("/input.csv", data = "<file>")]
async fn input_post(mut file: Form<Upload<'_>>) -> Result<Option<String>, Redirect> {
    let location = if let Some(path) = &file.file.path() {
        path.display().to_string()
    } else {
        if let Err(e) = file.file.persist_to("/tmp/test.bean").await {
            eprintln!("error {}", e);
            return Ok(None);
        }

        "/tmp/test.bean".to_string()
    };

    // bean-query -f csv main.bean 'select date, flag, account, number'
    let out = read_command(vec![
        "bean-query",
        "-f",
        "csv",
        &location,
        "select date, flag, account, number",
    ])
    .unwrap_or_default();

    Ok(Some(out))
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount("/fava/graphs", routes![index, input, input_post])
}
