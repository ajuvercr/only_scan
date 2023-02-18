use std::{convert::Infallible, ops::FromResidual};

use std::ops::{ControlFlow, Try};

use rocket::{
    request::{FromRequest, Outcome},
    response::Redirect,
    Request,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub user: String,
}

pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
impl<T, E> Result<T, E> {
    pub fn check(self) -> std::result::Result<T, E> {
        self.into()
    }
    pub fn check_err<EE>(self, e: EE) -> std::result::Result<T, EE> {
        match self {
            Result::Ok(x) => Ok(x),
            _ => Err(e),
        }
    }
}

impl<T, E, F: From<E>> FromResidual<Result<Infallible, E>> for Result<T, F> {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Result::Err(e) => Result::Err(From::from(e)),
            _ => panic!("Infallible not infallible"),
        }
    }
}

impl<T, E> Try for Result<T, E> {
    type Output = T;
    type Residual = Result<Infallible, E>;
    fn from_output(output: Self::Output) -> Self {
        Result::Ok(output)
    }
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Result::Ok(x) => ControlFlow::Continue(x),
            Result::Err(x) => ControlFlow::Break(Result::Err(x)),
        }
    }
}

impl<T, E> Into<std::result::Result<T, E>> for Result<T, E> {
    fn into(self) -> std::result::Result<T, E> {
        match self {
            Result::Ok(x) => Ok(x),
            Result::Err(y) => Err(y),
        }
    }
}

impl<T, E> From<std::result::Result<T, E>> for Result<T, E> {
    fn from(this: std::result::Result<T, E>) -> Self {
        match this {
            Ok(x) => Result::Ok(x),
            Err(e) => Result::Err(e),
        }
    }
}

pub type AuthUser = Result<User, Redirect>;
pub const COOKIE_NAME: &'static str = "scan_session";
pub const TOKEN_COOKIE_NAME: &'static str = "scan_key";

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(c) = req
            .cookies()
            .get(COOKIE_NAME)
            .and_then(|cookie| rocket::serde::json::from_str::<User>(cookie.value()).ok())
        {
            Outcome::Success(Result::Ok(c))
        } else {
            let host = req.guard().await.unwrap();
            let config = req.guard().await.unwrap();
            let url = req.uri().to_string();
            Outcome::Success(Result::Err(super::login(Some(&url), config, host)))
        }
    }
}
