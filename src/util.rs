use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use rocket::serde::json::serde_json::{self};
use rocket::serde::{Deserialize, Serialize};
use rocket::{Orbit, Rocket, State};
use std::fs;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Error {
    header: String,
    body: String,
}

impl Error {
    pub fn new(header: &str, body: &str) -> Self {
        Error {
            header: header.into(),
            body: body.into(),
        }
    }
}

pub fn get_mutexed_rocket<'a, T>(rocket: &'a Rocket<Orbit>) -> impl DerefMut<Target = T> + 'a
where
    T: Sync + Send + 'static,
{
    rocket
        .state::<Arc<Mutex<T>>>()
        .expect("No state found!")
        .lock()
        .expect("Failed unlock rocket state")
}

pub fn get_mutexed<'a, T>(state: &'a State<Arc<Mutex<T>>>) -> impl DerefMut<Target = T> + 'a
where
    T: Sync + Send + 'static,
{
    state.inner().lock().expect("Failed unlock rocket state")
}

pub async fn read_file<T>(loc: &str) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(loc).ok()?;

    serde_json::from_str(&content).ok()
}

// pub fn create_configurator<A, B, C, F>(
//     func: F,
// ) -> impl for<'b> Fn(&'b Rocket<Orbit>) -> BoxFuture<'b, ()>
// where
//     B: Send + Sync + 'static,
//     C: Future<Output = B> + Send + Sync + 'static,
//     F: FnMut(&A) -> C + Send + Sync + 'static,
//     A: Send + Sync + 'static,
// {
//     move |rocket| {
//         let f = &func;
//         Box::pin(async move {
//             let ff = &f;
//             if let Some(state) = rocket.state::<A>() {
//                 let foo = ff(state).await;
//             }
//         })
//     }
// }
