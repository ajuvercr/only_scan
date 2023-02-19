use std::path::PathBuf;

use rocket::response::Redirect;
use rocket::serde::json::serde_json::json;
use rocket::tokio::sync::oneshot;
use rocket::Build;
use rocket::Rocket;
use rocket::State;
use rocket_dyn_templates::Template;

mod post;
mod service;
pub use service::{BlogRequest, BlogService, BlogServiceClient};

use crate::context::Context;

#[derive(thiserror::Error, Debug, Clone)]
pub enum RPCError {
    #[error("failed to send rpc request")]
    RequestSend,
    #[error("failed to send rpc response")]
    ResponseSend,
    #[error("failed to receive rpc response")]
    ResponseReceive,
}

pub type HTML = String;
#[derive(thiserror::Error, Debug)]
pub enum BlogError {
    #[error("rpc error")]
    Disconnect(#[from] RPCError),
    #[error("Not found.")]
    NotFound,
    #[error("IO error.")]
    IO(std::io::ErrorKind),
    #[error("Notify error.")]
    Notify(#[from] notify::Error),
    #[error("Frontmatter error.")]
    Front(#[from] serde_yaml::Error),
}

pub struct RPC<A, R> {
    args: A,
    tx: oneshot::Sender<R>,
}

impl<A, R> RPC<A, R> {
    pub fn new(args: A) -> (Self, oneshot::Receiver<R>) {
        let (tx, rx) = oneshot::channel();
        (Self { args, tx }, rx)
    }
}

#[get("/")]
async fn get_blogs(service: &State<BlogServiceClient>) -> Option<Template> {
    None
}

#[get("/<uuid>")]
async fn get_blog(
    mut ctx: Context,
    uuid: &str,
    service: &State<BlogServiceClient>,
) -> Result<Template, Redirect> {
    let post = service.get(uuid).await.map_err(|_| Redirect::to("/blog"))?;
    let extra = json!({
        "post": post.as_ref(),
    });
    ctx.merge(extra);
    Ok(Template::render("blog/blog", &ctx.value()))
}

pub fn fuel(rocket: Rocket<Build>, base: PathBuf) -> (BlogService, Rocket<Build>) {
    let (service, client) = BlogService::new(base);
    (
        service,
        rocket
            .mount("/blog", routes!(get_blogs, get_blog))
            .manage(client),
    )
}
