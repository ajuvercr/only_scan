use std::{convert::TryInto, ffi::OsStr, io::Read, process::Command};
use rocket::fs::TempFile;
use rocket::form::Form;

#[macro_use] extern crate rocket;

#[derive(FromForm)]
struct Upload<'f> {
    upload: TempFile<'f>
}

fn read_command<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(cmd: I) -> Option<String> {
    let mut cmd_parts = cmd.into_iter();
    let first = cmd_parts.next()?;

    let output = Command::new(first).args(cmd_parts).output().ok()?;

    String::from_utf8(output.stdout).ok()
}

#[get("/")]
fn index() -> Option<String> {
    read_command(["ls", "/tmp"])
    // "Hello, world!"
}

#[post("/form", data = "<form>")]
async fn upload(mut form: Form<Upload<'_>>) -> Option<String> {
    form.upload.persist_to("/tmp/hallo.jpg").await.ok()?;
    let cmd = read_command(["tesseract", "/tmp/hallo.jpg", "-", "--psm", "0"])?;
    let mut lines = cmd.lines();
    let rotate = lines.find(|x| x.starts_with("Rotate:")).unwrap_or("Rotate: 0");
    let deg: isize = rotate.replace("Rotate:", "").trim().parse().ok()?;

    // convert test.jpg -rotate 90 -edge 10 test2.jpg
    let deg_s = deg.to_string();
    read_command(["convert", "/tmp/hallo.jpg", "-rotate", &deg_s, "-trim",  "-monochrome", "/tmp/hallo2.jpg"])?;

    read_command(["tesseract", "/tmp/hallo2.jpg", "-"])
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, upload])
}
