use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use rocket::fairing::AdHoc;
use rocket::serde::json::serde_json::{self};
use rocket::serde::{Deserialize, Serialize};
use std::fs;

pub mod db_repo;
use crate::util::*;

pub struct Repository<T> {
    inner: Arc<Mutex<T>>,
    location: String,
}

impl<T> Repository<T> {
    pub fn adhoc<F, C>(name: &'static str, func: F, default: T) -> AdHoc
    where
        T: Send + Sync + for<'de> Deserialize<'de> + Serialize + 'static,
        F: Fn(&C) -> String + Send + Sync + 'static,
        C: Send + Sync + 'static,
    {
        AdHoc::on_ignite(name, |rocket| {
            Box::pin(async move {
                let rocket = if let Some(config) = rocket.state::<C>() {
                    let t = func(config);
                    rocket.manage(Self::init_read(t, default).await)
                } else {
                    rocket
                };
                rocket
            })
        })
    }
}

macro_rules! get {
    ($i:expr) => {
        match ($i.lock()) {
            Err(e) => {
                println!("Poison error {}", e.to_string());
                panic!("aaaaaaahhhhhh");
            }
            Ok(t) => t,
        }
    };
}
impl<T> Repository<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    async fn init_read(location: String, default: T) -> Self {
        let inner = read_file::<T>(&location).await.unwrap_or(default);
        let out = Self {
            inner: Arc::new(Mutex::new(inner)),
            location,
        };

        out.save();

        out
    }

    fn save(&self) -> Option<()> {
        let vec = serde_json::to_vec_pretty(&self.inner.as_ref()).ok()?;
        fs::write(&self.location, &vec).ok()
    }

    pub fn with_save<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let out = {
            let mut t = get!(self.inner);
            // let mut t = self.inner.lock().expect("Failed unlock rocket state");
            func(t.deref_mut())
        };
        self.save();
        out
    }

    pub fn with<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut t = get!(self.inner);
        func(t.deref_mut())
    }
}
