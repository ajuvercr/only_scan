use std::ffi::OsStr;
use std::ops::DerefMut;
use std::process::Command;
use std::sync::{Arc, Mutex};

use rocket::serde::json::serde_json::{self};
use rocket::serde::{Deserialize, Serialize};
use rocket::{Orbit, Rocket, State};
use std::fs;

use crate::vision;

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

pub fn read_command<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(cmd: I) -> Option<String> {
    let mut cmd_parts = cmd.into_iter();
    let first = cmd_parts.next()?;

    let output = Command::new(first).args(cmd_parts).output().ok()?;

    String::from_utf8(output.stdout).ok()
}

pub fn turn_image(input: &str, output: &str) -> Option<()> {
    let cmd = read_command(["tesseract", input, "-", "--psm", "0"])?;

    let mut lines = cmd.lines();
    let rotate = lines
        .find(|x| x.starts_with("Rotate:"))
        .unwrap_or("Rotate: 0");
    let deg: isize = rotate.replace("Rotate:", "").trim().parse().ok()?;

    // convert test.jpg -rotate 90 -edge 10 test2.jpg
    let deg_s = deg.to_string();
    read_command([
        "convert",
        input,
        "-rotate",
        &deg_s,
        "-trim",
        "-monochrome",
        output,
    ])?;

    Some(())
}

// gcloud ml vision detect-document ./test.jpg
pub fn ocr(input: &str) -> Option<vision::Resp> {
    let str = read_command(["gcloud", "ml", "vision", "detect-document", input])?;

    serde_json::from_str(&str).ok()
}
