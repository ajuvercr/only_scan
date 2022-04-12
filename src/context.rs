use std::convert::Infallible;

use crate::oauth::{AResult, AuthUser};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::serde_json::Map;
use rocket::serde::json::{json, Value};

#[derive(Debug)]
pub struct Context {
    inner: Map<String, Value>,
}

fn to_map(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(x) => x,
        _ => panic!("No map"),
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Context {
    type Error = Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let loc = format!(
            "/{}",
            req.uri().path().segments().next().unwrap_or_default()
        );
        let routes = json! {[
            {"path": "/", "name": "Home"},
            {"path": "/scan", "name": "Scan"},
            {"path": "/scrum", "name": "Scrum"},
            {"path": "/fava", "name": "Fava", "subpaths": [
                {"path": "/fava/ingest", "name": "Ingest"},
                {"path": "/fava/beancount", "name": "Beancount"},
                {"path": "/fava/graphs", "name": "Graphs"},
            ]},

        ]};

        match req.guard::<AuthUser>().await {
            Outcome::Success(AResult::Ok(user)) => Outcome::Success(Self {
                inner: to_map(json! {{
                    "loc": loc,
                    "routes": routes,
                    "name": user.user,
                    "logged_in": true,
                }}),
            }),
            _ => Outcome::Success(Self {
                inner: to_map(json! {{
                    "loc": loc,
                    "routes": routes,
                    "logged_in": false,
                }}),
            }),
        }
    }
}

fn merge(target: &mut Map<String, Value>, from: Map<String, Value>) {
    for (k, v) in from {
        if !target.contains_key(&k) {
            target.insert(k, v);
            continue;
        }
        if v.is_object() {
            assert!(target[&k].is_object());
            merge(target[&k].as_object_mut().unwrap(), to_map(v));
        } else {
            target[&k] = v;
        }
    }
}

impl Context {
    pub fn merge(&mut self, obj: Value) {
        let map = to_map(obj);
        merge(&mut self.inner, map);
    }

    pub fn add<T: Into<Value>>(&mut self, key: &str, value: T) {
        self.inner.insert(key.to_string(), value.into());
    }

    pub fn value(self) -> Value {
        Value::Object(self.inner)
    }
}

impl Into<Value> for Context {
    fn into(self) -> Value {
        Value::Object(self.inner)
    }
}
