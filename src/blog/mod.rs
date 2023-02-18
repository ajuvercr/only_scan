use std::error::Error;
use std::future::Future;
use std::thread;

use rocket::tokio::sync::mpsc;
use rocket::tokio::sync::oneshot;
use rocket::Build;
use rocket::Rocket;
use rocket::State;
use serde::Deserialize;

pub struct HTML(pub String);

#[derive(Debug)]
struct OneShotSendError;

impl std::fmt::Display for OneShotSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to send to a oneshot channel")
    }
}

impl Error for OneShotSendError {}

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

pub trait Handler<A, R> {
    fn handle(&mut self, args: A) -> R;

    fn handle_rpc(&mut self, args: RPC<A, R>) -> Result<(), Box<dyn Error + Send>> {
        let res = self.handle(args.args);
        if args.tx.send(res).is_err() {
            return Err(Box::new(OneShotSendError));
        }
        Ok(())
    }
}

pub type BlogRequest = RPC<String, HTML>;

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
    pub async fn get(&self, path: &str) -> Result<HTML, OneShotSendError> {
        let (rpc, rx) = BlogRequest::new(path.to_string());
        self.tx.send(rpc).await.map_err(|_| OneShotSendError)?;
        rx.await.map_err(|_| OneShotSendError)
    }
}

pub struct BlogService {
    blog_request: mpsc::Receiver<BlogRequest>,
}

impl Handler<String, HTML> for BlogService {
    fn handle(&mut self, args: String) -> HTML {
        HTML(args)
    }
}

impl BlogService {
    pub fn new() -> (Self, BlogServiceClient) {
        let (tx, rx) = mpsc::channel(10);
        (Self { blog_request: rx }, BlogServiceClient { tx })
    }

    pub async fn start(mut self) -> Result<(), Box<dyn Error + Send>> {
        while let Some(x) = self.blog_request.recv().await {
            self.handle_rpc(x)?;
        }

        Ok(())
    }
}

#[get("/<uuid>")]
async fn get_blog(uuid: &str, service: &State<BlogServiceClient>) -> Result<String, &'static str> {
    let file = service.get(uuid).await.map_err(|_| "Failed")?;
    Ok(file.0)
}

pub fn fuel(rocket: Rocket<Build>) -> (BlogService, Rocket<Build>) {
    let (service, client) = BlogService::new();
    (
        service,
        rocket.mount("/blog", routes!(get_blog)).manage(client),
    )
}
