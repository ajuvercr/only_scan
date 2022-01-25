use feignhttp::feign;
use rocket::{
    http::{Cookie, CookieJar},
    response::Redirect,
    serde::json::serde_json,
    Build, Rocket,
};
use serde::{Deserialize, Serialize};

mod user;
pub use user::{User, AuthUser, Result as AResult};

const ZAUTH_URL: &str = "http://localhost:8001";

#[derive(Debug, Serialize, Deserialize)]
struct TokenReq {
    grant_type: String,
    code: String,
    redirect_uri: String,
    client_id: String,
    client_secret: String,
}

impl TokenReq {
    fn new(code: String, redirect_uri: String) -> Self {
        Self {
            code,
            redirect_uri,
            grant_type: String::from("authorization_code"),
            client_id: String::from("only-scan"),
            client_secret: String::from(
                "Y5xSinM49kNIsw3Tcn02pTXYHO2YED9zRqJTJukzwMGp68lajc34kArNSHPzcWHq",
            ),
        }
    }
}

const LOGIN_URL: &'static str ="http://localhost:8001/oauth/authorize?response_type=code&client_id=only-scan&redirect_uri=http://localhost:8000/oauth/callback&state=bla";

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResp {
    access_token: String,
    token_type: String,
    expires_in: i64,
    info: User,
}

struct Oauth;

#[feign(url = ZAUTH_URL)]
impl Oauth {
    #[post("/oauth/token")]
    async fn repository(#[form] body: TokenReq) -> feignhttp::Result<TokenResp> {}
}

#[derive(Debug, FromForm, Serialize, Deserialize)]
pub struct Callback {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
}

#[get("/callback?<data..>")]
async fn callback<'r>(data: Callback, jar: &CookieJar<'_>) -> Option<String> {
    println!("got callback {:?}", data);
    let resp = Oauth::repository(TokenReq::new(
        data.code.unwrap(),
        "http://localhost:8000/oauth/callback".to_string(),
    ))
    .await
    .ok()?;

    println!("resp {:?}", resp);

    jar.add_private(Cookie::new(
        user::COOKIE_NAME,
        serde_json::to_string(&resp.info).ok()?,
    ));

    Some(format!("{:?}", resp))
}

#[get("/login")]
pub fn login() -> Redirect {
    Redirect::to(LOGIN_URL)
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount("/oauth", routes![callback, login,])
}
