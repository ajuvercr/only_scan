use rocket::{Request, Data, Response};
use rocket::fairing::{Fairing, Info, Kind};


pub struct Debug;

#[rocket::async_trait]
impl Fairing for Debug {
    fn info(&self) -> Info {
        Info {
            name: "GET/POST Counter",
            kind: Kind::Request | Kind::Response
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, data: &mut Data<'_>) {
        let body = data.peek(512).await;
        let cont = std::str::from_utf8(body).unwrap_or("Failed to parse body as str.");
        println!("req: to {}", request.uri());
        println!("{}", cont);
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        println!("resp to {}", request.uri());
        println!("{:?}", response);
    }
}

