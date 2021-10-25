#[macro_use]
extern crate rocket;
extern crate chrono;
extern crate rand;
extern crate rocket_dyn_templates;
extern crate uuid;
extern crate regex;

mod desk;
pub mod repository;
mod scan;
mod serve;
pub mod sorted_list;
pub mod util;
pub mod vision;

#[cfg(test)]
mod tests;

use rocket::Route;
use rocket_dyn_templates::Template;

use rocket::serde::Serialize;

use rocket_dyn_templates::handlebars::{
    Context, Handlebars, Helper, HelperResult, JsonRender, Output, RenderContext,
};

#[get("/")]
async fn index() -> Template {
    #[derive(Serialize)]
    struct IndexContext {
        firstname: String,
        lastname: String,
    }

    let context = IndexContext {
        firstname: String::from("Arthur"),
        lastname: String::from("Meeee"),
    };

    Template::render("index", &context)
}

fn another_simple_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0).unwrap();

    let input: String = param.value().render();

    let pretty = if let Some(rfind) = input.rfind(':') {
        input.get(rfind + 1..).unwrap()
    } else {
        input.as_ref()
    };

    out.write(pretty)?;
    Ok(())
}

#[launch]
fn rocket() -> _ {
    let statics: Vec<Route> = serve::StaticFiles::new("static", serve::Options::DotFiles).into();

    let rocket = rocket::build()
        .mount("/", routes![index])
        .mount("/", statics);
    let rocket = desk::fuel(rocket);
    let rocket = scan::fuel(rocket);

    // This also adds the handlebars fairing
    rocket.attach(Template::custom(|engines| {
        engines
            .handlebars
            .register_helper("length", Box::new(another_simple_helper));
    }))
}
