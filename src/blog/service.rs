use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use notify::event::ModifyKind;
use notify::recommended_watcher;
use notify::EventKind;
use notify::RecursiveMode;
use rocket::futures::future::BoxFuture;
use rocket::futures::FutureExt;
use rocket::tokio::fs;
use rocket::tokio::sync::mpsc;

use super::post::APost;
use super::post::Post;
use super::post::PostShort;
use super::{BlogError, RPCError, RPC};

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

/// BlogRequest is a RPC call that takes in a String and tries to return a HTML element
pub type BlogRequest = RPC<String, Result<APost, BlogError>>;

pub type AShorts = Arc<Vec<PostShort>>;
pub type BlogIndexRequest = RPC<(), Result<AShorts, BlogError>>;

/// Client that interacts with the actual BlogService
pub struct BlogServiceClient {
    blog_request: mpsc::Sender<BlogRequest>,
    index_request: mpsc::Sender<BlogIndexRequest>,
}

impl BlogServiceClient {
    pub async fn get(&self, path: &str) -> Result<APost, BlogError> {
        let (rpc, rx) = BlogRequest::new(path.to_string());
        self.blog_request
            .send(rpc)
            .await
            .map_err(|_| RPCError::RequestSend)?;
        rx.await.map_err(|_| RPCError::ResponseReceive)?
    }
    pub async fn index(&self) -> Result<Arc<Vec<PostShort>>, BlogError> {
        let (rpc, rx) = BlogIndexRequest::new(());
        self.index_request
            .send(rpc)
            .await
            .map_err(|_| RPCError::RequestSend)?;
        rx.await.map_err(|_| RPCError::ResponseReceive)?
    }
}

/// The actual Blog service
///
/// Keeps the posts in sync with the filesystem, and answers RPC requests
pub struct BlogService {
    base: PathBuf,
    posts: HashMap<PathBuf, Arc<Post>>,
    shorts: AShorts,
    blog_request: mpsc::Receiver<BlogRequest>,
    index_request: mpsc::Receiver<BlogIndexRequest>,
}

impl BlogService {
    pub fn new(base: PathBuf) -> (Self, BlogServiceClient) {
        let (tx, rx) = mpsc::channel(10);
        let (itx, irx) = mpsc::channel(10);
        (
            Self {
                blog_request: rx,
                index_request: irx,
                posts: HashMap::new(),
                shorts: Arc::new(Vec::new()),
                base,
            },
            BlogServiceClient {
                index_request: itx,
                blog_request: tx,
            },
        )
    }

    fn calc_shorts(&mut self) {
        let mut shorts: Vec<_> = self
            .posts
            .iter()
            .map(|(key, value)| {
                let path = key.strip_prefix(&self.base).unwrap();
                PostShort::new(value, format!("/blog/{}", path.display()))
            })
            .collect();
        shorts.sort_by(|a, b| a.date.cmp(&b.date).reverse());
        self.shorts = Arc::new(shorts);
    }

    fn check_fut<'a, 'b: 'a>(&'a mut self, path: &'b Path) -> BoxFuture<Result<(), BlogError>> {
        async move { self.check(path).await }.boxed()
    }

    async fn check(&mut self, path: &Path) -> Result<(), BlogError> {
        if path.is_dir() {
            let mut dir = fs::read_dir(path)
                .await
                .map_err(|x| BlogError::IO(x.kind()))?;
            while let Ok(Some(p)) = dir.next_entry().await {
                self.check_fut(&p.path()).await?;
            }
        } else {
            if path.exists() {
                let post = Post::load(&path).await?;
                self.posts.insert(path.to_path_buf(), Arc::new(post));
            } else {
                self.posts.remove(path);
            }
        }
        Ok(())
    }

    async fn handle_file_event(&mut self, event: notify::event::Event) {
        match event.kind {
            EventKind::Modify(ModifyKind::Data(_))
            | EventKind::Create(_)
            | EventKind::Remove(_) => {
                println!("handling event {:?}", event);
                for path in event.paths {
                    if let Err(e) = self.check(&path).await {
                        eprintln!("{}", e);
                    }
                }
            }
            _ => {
                println!("unhandled event {:?}", event);
            }
        }
        self.calc_shorts();
    }

    async fn inner_start(mut self) -> Result<(), BlogError> {
        use notify::Watcher;
        let (file_tx, mut file_rx) = mpsc::channel(10);
        let mut watcher = recommended_watcher(move |x| file_tx.blocking_send(x).unwrap())?;
        watcher.watch(&self.base, RecursiveMode::Recursive)?;

        let base = self.base.clone();
        self.check(&base).await?;
        self.calc_shorts();

        loop {
            rocket::tokio::select! {
                req = self.blog_request.recv() => {
                    if let Some(x) = req {
                        self.handle_rpc(x).await?;
                    } else {
                        break;
                    }
                },
                req = self.index_request.recv() => {
                    if let Some(req) = req {
                        self.handle_rpc(req).await?;
                    } else {
                        break;
                    }
                },
                req = file_rx.recv() => {
                    if let Some(Ok(x)) = req {
                        self.handle_file_event(x).await;
                    }
                },

            };
        }

        Ok(())
    }

    pub async fn start(self) {
        if let Err(err) = self.inner_start().await {
            eprintln!("{}", err);
        }
    }
}

#[async_trait]
impl Handler<String, Result<APost, BlogError>> for BlogService {
    async fn handle(&mut self, args: String) -> Result<APost, BlogError> {
        let path = self.base.join(args);

        if let Some(x) = self.posts.get(&path) {
            return Ok(x.clone());
        } else {
            return Err(BlogError::NotFound);
        }
    }
}

#[async_trait]
impl Handler<(), Result<AShorts, BlogError>> for BlogService {
    async fn handle(&mut self, _: ()) -> Result<AShorts, BlogError> {
        Ok(self.shorts.clone())
    }
}
