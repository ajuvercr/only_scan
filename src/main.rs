#[macro_use]
extern crate rocket;
extern crate chrono;
extern crate cool_id_generator;
extern crate rand;
extern crate regex;
extern crate rocket_dyn_templates;
extern crate time;
extern crate uuid;

mod desk;
pub mod repository;
mod scan;
mod scrum;
mod serve;
pub mod sorted_list;
pub mod util;
pub mod vision;

#[cfg(test)]
mod tests;

use rocket::Route;
use rocket_dyn_templates::{Template, handlebars::{handlebars_helper}};

#[get("/")]
async fn index() -> Template {
    Template::render("index", ())
}

handlebars_helper!(shorten_cat: |x: str|
    x.rfind(':').and_then(|rfind| x.get(rfind + 1..)).unwrap_or(x)
);
handlebars_helper!(into_euro: |x: u64| format!("{:.2}", x as f64 / 100.0));
handlebars_helper!(eq: |x: str, y: str| x == y);
handlebars_helper!(lower: |x: str| x.to_lowercase());
handlebars_helper!(has_status_next: |x: str| scrum::has_next(x));
handlebars_helper!(has_status_previous: |x: str| scrum::has_previous(x));
handlebars_helper!(image: |name: Json| {
    if let rocket::serde::json::Value::String(name) = name {
        format!("/static/{}", name)
    } else {
        String::from("/static/images/default.png")

    }
});

#[launch]
fn rocket() -> _ {
    let statics: Vec<Route> = serve::StaticFiles::new("static", serve::Options::DotFiles).into();

    let rocket = rocket::build()
        .mount("/", routes![index])
        .mount("/static", statics);
    let rocket = desk::fuel(rocket);
    let rocket = scan::fuel(rocket);
    let rocket = scrum::fuel(rocket);
    // This also adds the handlebars fairing
    rocket.attach(Template::custom(|engines| {
        let handles = &mut engines.handlebars;
        handles.register_helper("eq", Box::new(eq));
        handles.register_helper("shorten_cat", Box::new(shorten_cat));
        handles.register_helper("euro", Box::new(into_euro));
        handles.register_helper("lower", Box::new(lower));
        handles.register_helper("image", Box::new(image));

        handles.register_helper("has_status_next", Box::new(has_status_next));
        handles.register_helper("has_status_previous", Box::new(has_status_previous));
    }))
}
