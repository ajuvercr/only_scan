use std::path::PathBuf;

use pulldown_cmark::html;
use pulldown_cmark::Options;
use pulldown_cmark::Parser;
use rocket::response::Redirect;
use rocket::tokio::fs;
use rocket::tokio::sync::mpsc;
use rocket::tokio::sync::oneshot;
use rocket::Build;
use rocket::Rocket;
use rocket::State;
use rocket_dyn_templates::Template;
use serde::Deserialize;

#[derive(thiserror::Error, Debug, Clone)]
pub enum RPCError {
    #[error("failed to send rpc request")]
    RequestSend,
    #[error("failed to send rpc response")]
    ResponseSend,
    #[error("failed to receive rpc response")]
    ResponseReceive,
}

pub struct HTML(pub String);
#[derive(thiserror::Error, Debug, Clone)]
pub enum BlogError {
    #[error("rpc error")]
    Disconnect(#[from] RPCError),
    #[error("Internal error.")]
    Internal(String),
    #[error("Not found.")]
    NotFound,
    #[error("IO error.")]
    IO(std::io::ErrorKind),
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

use async_trait::async_trait;

use crate::context::Context;

#[async_trait]
pub trait Handler<A: Send + 'static, R: Send + 'static> {
    async fn handle(&mut self, args: A) -> R;

    async fn handle_rpc(&mut self, args: RPC<A, R>) -> Result<(), BlogError> {
        let res = self.handle(args.args).await;
        if args.tx.send(res).is_err() {
            return Err(RPCError::ResponseSend.into());
        }
        Ok(())
    }
}

pub type BlogRequest = RPC<String, Result<HTML, BlogError>>;

#[derive(Deserialize)]
pub struct FrontMatter {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    draft: bool,
}

pub struct BlogServiceClient {
    tx: mpsc::Sender<BlogRequest>,
}

impl BlogServiceClient {
    pub async fn get(&self, path: &str) -> Result<HTML, BlogError> {
        let (rpc, rx) = BlogRequest::new(path.to_string());
        self.tx.send(rpc).await.map_err(|_| RPCError::RequestSend)?;
        rx.await.map_err(|_| RPCError::ResponseReceive)?
    }
}

pub struct BlogService {
    base: PathBuf,
    blog_request: mpsc::Receiver<BlogRequest>,
}
#[async_trait]
impl Handler<String, Result<HTML, BlogError>> for BlogService {
    async fn handle(&mut self, args: String) -> Result<HTML, BlogError> {
        let path = self.base.join(args);

        if !path.exists() || !path.is_file() {
            return Err(BlogError::NotFound);
        }

        let contents = fs::read_to_string(path)
            .await
            .map_err(|e| BlogError::IO(e.kind()))?;

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(&contents, options);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        Ok(HTML(html_output))
    }
}

impl BlogService {
    pub fn new(base: &str) -> (Self, BlogServiceClient) {
        let (tx, rx) = mpsc::channel(10);
        (
            Self {
                blog_request: rx,
                base: base.into(),
            },
            BlogServiceClient { tx },
        )
    }

    pub async fn start(mut self) -> Result<(), BlogError> {
        while let Some(x) = self.blog_request.recv().await {
            self.handle_rpc(x).await?;
        }

        Ok(())
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
    let html = service.get(uuid).await.map_err(|_| Redirect::to("/blog"))?;
    ctx.add("content", html.0);
    Ok(Template::render("blog/blog", &ctx.value()))
}

pub fn fuel(rocket: Rocket<Build>, base: &str) -> (BlogService, Rocket<Build>) {
    let (service, client) = BlogService::new(base);
    (
        service,
        rocket
            .mount("/blog", routes!(get_blogs, get_blog))
            .manage(client),
    )
}
