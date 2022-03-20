use std::{collections::HashMap, sync::Arc};

use rocket::{
    futures::lock::Mutex,
    http::{Cookie, CookieJar},
    response::Redirect,
    routes,
    serde::json::serde_json,
    Build, Rocket, State,
};
use serde::{Deserialize, Serialize};

#[macro_use]
pub mod user;
pub use user::{AuthUser, Result as AResult, User};

type Routes = Arc<Mutex<HashMap<String, String>>>;

#[derive(Debug, Serialize, Deserialize)]
struct TokenReq {
    grant_type: String,
    code: String,
    redirect_uri: String,
    client_id: String,
    client_secret: String,
}

impl TokenReq {
    fn new(code: String, redirect_uri: String, config: &util::Config) -> Self {
        Self {
            code,
            redirect_uri,
            grant_type: String::from("authorization_code"),
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResp {
    access_token: String,
    token_type: String,
    expires_in: i64,
    info: User,
}

use feignhttp::post as fpost;

use crate::util::{self, HostHeader};
#[fpost("{url}/oauth/token")]
async fn token_req(#[path] url: &str, #[form] body: TokenReq) -> feignhttp::Result<TokenResp> {}

#[derive(Debug, FromForm, Serialize, Deserialize)]
pub struct Callback {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
}

#[get("/callback?<data..>")]
async fn callback<'r>(
    data: Callback,
    jar: &CookieJar<'_>,
    config: &State<util::Config>,
    host: HostHeader<'r>,
) -> Option<String> {
    println!("callback: hostheader {:?}", host);

    let resp = token_req(
        &config.inner().oauth_base,
        TokenReq::new(
            data.code.unwrap(),
            format!("{}/oauth/callback", host.get()),
            &config,
        ),
    )
    .await
    .ok()?;

    jar.add_private(Cookie::new(
        user::COOKIE_NAME,
        serde_json::to_string(&resp.info).ok()?,
    ));

    Some(format!("{:?}", resp))
}

#[get("/login?<from>")]
pub fn login(from: &str, config: &State<util::Config>, host: HostHeader<'_>) -> Redirect {
    let state = base64::decode_config(from, base64::URL_SAFE).and(String::from_utf8).unwrap_or("".to_string());

    if let Ok(callback) = base64::decode_config(from, base64::URL_SAFE) {}
    let url = format!("{}/oauth/authorize?response_type=code&client_id={}&redirect_uri={}/oauth/callback&state={}",
                      config.oauth_base, config.client_id, host.get(),state );
    println!("login url: {}", url);
    Redirect::to(url).into()
}

#[get("/logout")]
fn logout(cookies: &CookieJar) -> Redirect {
    cookies.remove(Cookie::named(user::COOKIE_NAME));
    // cookies.remove(Cookie::named());
    Redirect::to("/")
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount("/oauth", routes![callback, login, logout,])
}
