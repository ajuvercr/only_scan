#[macro_use]
extern crate rocket;
extern crate rocket_dyn_templates;
extern crate uuid;

mod desk;
pub mod sorted_list;

use rocket_dyn_templates::Template;

use rocket::fs::{NamedFile, TempFile};
use std::path::{Path, PathBuf};
use std::{ffi::OsStr, process::Command};

use rocket::serde::{json::Json, Deserialize, Serialize};

// use rocket::serde::{Serialize, Deserialize};
// use rocket::json::Json;

#[derive(FromForm)]
struct Upload<'f> {
    upload: TempFile<'f>,
}

fn read_command<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(cmd: I) -> Option<String> {
    let mut cmd_parts = cmd.into_iter();
    let first = cmd_parts.next()?;

    let output = Command::new(first).args(cmd_parts).output().ok()?;

    String::from_utf8(output.stdout).ok()
}

// #[get("/")]
// fn index() -> Redirect {
//     Redirect::to("static/index.html")
// }

async fn file_to_text(mut file: TempFile<'_>) -> Option<String> {
    file.persist_to("/tmp/hallo.jpg").await.ok()?;

    println!("Saved file!");
    let cmd = read_command(["tesseract", "/tmp/hallo.jpg", "-", "--psm", "0"])?;

    let mut lines = cmd.lines();
    let rotate = lines
        .find(|x| x.starts_with("Rotate:"))
        .unwrap_or("Rotate: 0");
    let deg: isize = rotate.replace("Rotate:", "").trim().parse().ok()?;

    // convert test.jpg -rotate 90 -edge 10 test2.jpg
    let deg_s = deg.to_string();
    read_command([
        "convert",
        "/tmp/hallo.jpg",
        "-rotate",
        &deg_s,
        "-trim",
        "-monochrome",
        "/tmp/hallo2.jpg",
    ])?;

    read_command(["tesseract", "--psm", "6", "/tmp/hallo2.jpg", "-"])
}

#[post("/upload", data = "<file>")]
async fn upload(file: TempFile<'_>) -> Json<Results> {
    if let Some(inner) = file_to_text(file)
        .await
        .as_ref()
        .map(String::as_str)
        .map(parse_texts)
    {
        Json(Results::Success(inner))
    } else {
        Json(Results::Error("To baddd".to_string()))
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Results {
    Success(Vec<Item>),
    Error(String),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    name: String,
    price: f32,
}

enum ItemState {
    Centime,
    Euro,
    Name,
}

pub fn parse_texts(input: &str) -> Vec<Item> {
    input.lines().filter_map(parse_text).collect()
}

pub fn parse_text(input: &str) -> Option<Item> {
    println!("Current {}", input);
    let mut price: Vec<u8> = Vec::with_capacity(input.len());
    let mut name: Vec<u8> = Vec::with_capacity(input.len());

    let mut doing_price = ItemState::Centime;

    for c in input.chars().rev() {
        match doing_price {
            ItemState::Centime => {
                if c.is_whitespace() {
                    continue;
                }

                if c == ',' || c == '.' {
                    continue;
                }

                if price.len() >= 2 {
                    doing_price = ItemState::Euro;
                }

                if c.is_ascii_digit() {
                    price.push(c as u8);
                } else {
                    eprintln!("{} was unexpected in {}", c, input);
                    return None;
                }
            }
            ItemState::Euro => {
                if c.is_whitespace() {
                    continue;
                }

                if price.len() == 2 && (c == ',' || c == '.') {
                    continue;
                }

                if c.is_ascii_digit() {
                    price.push(c as u8);
                } else {
                    name.push(c as u8);
                    doing_price = ItemState::Name;
                }
            }
            ItemState::Name => {
                name.push(c as u8);
            }
        }
    }

    if price.len() < 3 {
        return None;
    }

    price.reverse();
    name.reverse();

    let price_str = String::from_utf8_lossy(&price);

    Item {
        name: String::from_utf8_lossy(&name).into(),
        price: price_str.parse::<f32>().ok()? / 100.0,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing() {
        if let Some(item) = parse_text("test 13.20\n") {
            assert_eq!(item.name, "test");
            assert_eq!(item.price, 13.2);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_alpro() {
        if let Some(item) = parse_text("1L ALP DRINK AMAND 2 29 \n") {
            assert_eq!(item.name, "1L ALP DRINK AMAND");
            assert_eq!(item.price, 2.29);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_vuilzak_groen() {
        if let Some(item) = parse_text("VUILZAK GROEN 30L - 1110 \n") {
            assert_eq!(item.name, "VUILZAK GROEN 30L -");
            assert_eq!(item.price, 11.10);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_fail_1() {
        assert_eq!(parse_text("  \n"), None);
    }

    #[test]
    fn test_fail_2() {
        assert_eq!(parse_text("50CL. MNSTR PARADIS 1f42 "), None);
    }
}

#[get("/")]
fn index() -> Template {
    #[derive(Serialize)]
    struct IndexContext {
        firstname: String,
        lastname: String,
    }

    let context = IndexContext {
        firstname: String::from("Arthur"),
        lastname: String::from("Meeee"),
    };

    Template::render("index", &context)
}

#[get("/<path..>")]
pub async fn files(path: PathBuf) -> Option<NamedFile> {
    let mut path = Path::new("static").join(path);
    if path.is_dir() {
        path.push("index.html");
    }

    NamedFile::open(path).await.ok()
}

#[launch]
fn rocket() -> _ {
    let rocket = rocket::build().mount("/", routes![index, upload, files]);
    let rocket = desk::fuel(rocket);

    rocket.attach(Template::fairing())
}
