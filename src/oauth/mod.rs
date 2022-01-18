use feignhttp::feign;
use rocket::{response::Redirect, Build, Rocket};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize)]
struct TokenResp {
    access_token: String,
    token_type: String,
    expires_in: i64,
}

struct Oauth;

#[feign(url = ZAUTH_URL)]
impl Oauth {
    #[post("/oauth/token")]
    async fn repository(#[form] body: TokenReq) -> feignhttp::Result<String> {}
}

#[derive(Debug, FromForm, Serialize, Deserialize)]
pub struct Callback {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
}

#[get("/callback?<data..>")]
async fn callback<'r>(data: Callback) -> Option<String> {
    println!("got callback {:?}", data);
    let resp = Oauth::repository(TokenReq::new(
        data.code.unwrap(),
        "http://localhost:8000/oauth/callback".to_string(),
    ))
    .await;
    println!("resp {:?}", resp);
    // Lets do some code exchange please!
    Some(format!("{:?}", resp))
}

#[get("/login")]
fn login() -> Redirect {
    let uri = uri!("http://localhost:8001/oauth/authorize?response_type=code&client_id=only-scan&redirect_uri=http://localhost:8000/oauth/callback&state=bla");

    Redirect::to(uri)
}

pub fn fuel(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount("/oauth", routes![callback, login,])
}
