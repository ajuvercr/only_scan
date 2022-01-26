#![recursion_limit = "256"]
#![feature(try_trait_v2)]
#![feature(const_fn_trait_bound)]
#[macro_use]
extern crate rocket;
extern crate chrono;
#[macro_use]
extern crate crud_helper;
#[macro_use]
extern crate diesel;
extern crate cool_id_generator;
extern crate rand;
extern crate regex;
extern crate rocket_dyn_templates;
extern crate time;
extern crate uuid;

extern crate feignhttp;

mod debug;
mod desk;
pub mod oauth;
pub mod repository;
mod scan;
mod scrum;
mod serve;
pub mod sorted_list;
pub mod util;
pub mod vision;

#[cfg(test)]
mod tests;

use r2d2_diesel::ConnectionManager;
use rocket::{
    fairing::AdHoc,
    http::Status,
    request::{self, FromRequest, Outcome},
    Request, Route, State,
};
use rocket_dyn_templates::{handlebars::handlebars_helper, Template};

pub type Conn = diesel::PgConnection;
pub type Pool = r2d2::Pool<ConnectionManager<Conn>>;

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
handlebars_helper!(image: |name: Json| {
    if let rocket::serde::json::Value::String(name) = name {
        format!("/static/{}", name)
    } else {
        String::from("/static/images/default.png")

    }
});

pub fn init_pool(config: &util::Config) -> Pool {
    let manager = ConnectionManager::new(&config.database_url);
    Pool::new(manager).expect("db pool failed")
}

pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<Conn>>);

impl std::ops::Deref for DbConn {
    type Target = Conn;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DbConn {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DbConn {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let pool: &State<Pool> = req.guard().await.unwrap();
        match pool.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}

#[launch]
fn rocket() -> _ {
    let statics: Vec<Route> = serve::StaticFiles::new("static", serve::Options::DotFiles).into();
    let rocket = rocket::build();
    let config: util::Config = rocket.figment().extract().expect("config");

    let rocket = rocket
        .mount("/", routes![index])
        .mount("/static", statics)
        .attach(AdHoc::config::<util::Config>())
        .manage(init_pool(&config));
    let rocket = desk::fuel(rocket);
    let rocket = scan::fuel(rocket);
    let rocket = scrum::fuel(rocket);
    let rocket = oauth::fuel(rocket);
    // This also adds the handlebars fairing

    rocket.attach(Template::custom(|engines| {
        let handles = &mut engines.handlebars;
        handles.register_helper("eq", Box::new(eq));
        handles.register_helper("shorten_cat", Box::new(shorten_cat));
        handles.register_helper("euro", Box::new(into_euro));
        handles.register_helper("lower", Box::new(lower));
        handles.register_helper("image", Box::new(image));
    }))
    //        .attach(debug::Debug)
}
