use rocket::form::{Contextual, Form};
use rocket::fs::{NamedFile, TempFile};
use rocket::response::Redirect;
use std::path::{Path, PathBuf};
use std::{convert::TryInto, ffi::OsStr, io::Read, process::Command};

#[macro_use]
extern crate rocket;

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

#[post("/upload", data = "<file>")]
async fn upload(mut file: TempFile<'_>) -> Option<String> {
    file.persist_to("/tmp/hallo.jpg").await.ok()?;
    let cmd = read_command(["tesseract", "/tmp/hallo.jpg", "-", "--psm", "0"])?;

    println!("cmd\n{}",cmd);
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

    read_command(["tesseract", "/tmp/hallo2.jpg", "-"])
}

#[post("/upload2", data = "<form>")]
fn submit(form: Form<Contextual<'_, Upload<'_>>>) {
    if let Some(ref value) = form.value {
        // The form parsed successfully. `value` is the `T`.
        println!("successful!");
    }

    // We can retrieve raw field values and errors.
    let raw_id_value = form.context.field_value("upload");

    println!("raw id value: {:?}", raw_id_value);
    for e in form.context.field_errors("upload") {
        println!("id error {:?}", e);
    }

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
    rocket::build().mount("/", routes![upload, submit, files])
}
