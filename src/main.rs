#![recursion_limit = "256"]
#![feature(try_trait_v2)]
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate lazy_static;
extern crate base64;
extern crate chrono;

extern crate crud_helper;
#[cfg(feature = "db")]
#[macro_use]
extern crate diesel;

extern crate cool_id_generator;
extern crate rand;
extern crate regex;
extern crate rocket_dyn_templates;
extern crate time;
extern crate uuid;

extern crate feignhttp;

#[macro_use]
pub mod oauth;
mod context;
mod debug;
mod fava;
mod pages;
pub mod repository;
pub mod sorted_list;
pub mod util;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use context::Context;
use rocket::{
    fairing::AdHoc,
    fs::{FileServer, Options},
    routes, Route,
};
use rocket_dyn_templates::{handlebars::handlebars_helper, Template};

#[get("/")]
async fn index(context: Context) -> Template {
    Template::render("index", context.value())
}

lazy_static! {
    static ref EXAMPLE: HashMap<&'static str, &'static str> = HashMap::from([
        ("Eten", "#f1d3a1"),
        ("Nut", "#e3dbd9"),
        ("Wonen", "#e6eff6"),
        ("Transport", "#89b4c4"),
        ("Uitgaven", "#548999"),
        ("Afhaal", "#C38AF2"),
    ]);
}

handlebars_helper!(shorten_cat: |x: str|
    x.rfind(':').and_then(|rfind| x.get(rfind + 1..)).unwrap_or(x)
);
handlebars_helper!(color_cat: |x: str| {
    let mut parts = x.split(':');
    let try_one = parts.next_back().and_then(|x| EXAMPLE.get(x));
    let llast = parts.next_back().and_then(|x| EXAMPLE.get(x));
    try_one.or(llast).map(|&x| x).unwrap_or("#BBF2D0")
});

handlebars_helper!(into_euro: |x: i64| format!("{:.2}", x as f64 / 100.0));
handlebars_helper!(eq: |x: str, y: str| x == y);
handlebars_helper!(lower: |x: str| x.to_lowercase());
handlebars_helper!(image: |name: Json| {
    if let rocket::serde::json::Value::String(name) = name {
        format!("/static/{}", name)
    } else {
        String::from("/static/images/default.png")

    }
});

#[launch]
fn rocket() -> _ {
    let statics: Vec<Route> = FileServer::new("static", Options::DotFiles).into();
    let rocket = rocket::build();

    let rocket = rocket
        .mount("/", routes![index])
        .mount("/static", statics)
        .attach(AdHoc::config::<util::Config>());

    #[cfg(feature = "db")]
    let rocket = {
        let config: util::Config = rocket.figment().extract().expect("config");
        let rocket = rocket.manage(repository::db_repo::init_pool(&config));
        pages::scrum::fuel(rocket)
    };

    let rocket = pages::desk::fuel(rocket);
    let rocket = oauth::fuel(rocket);
    let rocket = fava::fuel(rocket);
    // This also adds the handlebars fairing

    rocket
        .attach(Template::custom(|engines| {
            let handles = &mut engines.handlebars;
            handles.register_helper("eq", Box::new(eq));
            handles.register_helper("shorten_cat", Box::new(shorten_cat));
            handles.register_helper("color_cat", Box::new(color_cat));
            handles.register_helper("euro", Box::new(into_euro));
            handles.register_helper("lower", Box::new(lower));
            handles.register_helper("image", Box::new(image));
        }))
        .attach(debug::Debug)
}
