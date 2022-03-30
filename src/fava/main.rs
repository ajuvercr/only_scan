#[macro_use]
extern crate rocket;
extern crate csv;
use chrono::NaiveDate;
use regex::Regex;
use std::io::Cursor;

use rocket::data::Data;
use rocket::data::ToByteUnit;
use rocket::response::content::Html;

use serde::{de, Deserialize, Serialize};

mod my_date;

#[derive(Serialize, Deserialize, Debug)]
struct Note {
    #[serde(rename = "gestructureerde mededeling")]
    #[serde(deserialize_with = "my_date::deserialize_spacy_string")]
    structured: Option<String>,
    #[serde(rename = "Vrije mededeling")]
    #[serde(deserialize_with = "my_date::deserialize_spacy_string")]
    free: Option<String>,
}

type Time = String;
type Utencil = String;

#[derive(Debug)]
struct Dom {
    way: String,
    label: String,
}

#[derive(Debug)]
struct Bet {
    way: String,
    label: String,
    user: String,
}

#[derive(Debug)]
enum Description {
    Domiciliering(Dom),
    Betaling(Bet),
}



impl FromStr for Description {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("") {

        } else {

        }
    }
}



#[derive(Serialize, Deserialize, Debug)]
struct Statement {
    #[serde(rename = "Omschrijving")]
    #[serde(deserialize_with = "my_date::deserialize_spacy_string")]
    description: Option<String>,
    #[serde(rename = "Bedrag")]
    amount: f32,
    #[serde(rename = "Datum")]
    #[serde(with = "my_date")]
    date: NaiveDate,
    #[serde(rename = "Naam tegenpartij")]
    #[serde(deserialize_with = "my_date::deserialize_spacy_string")]
    tegenpartij: Option<String>,
    #[serde(flatten)]
    node: Note,
}

fn deserialize_spacy_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let re = Regex::new(r"\s+").unwrap();
    let s: String = de::Deserialize::deserialize(deserializer)?;
    let st: String = re.replace_all(&s, " ").as_ref().trim().to_string();
    Ok(if st.is_empty() { None } else { Some(st) })
}

#[get("/")]
fn index() -> Html<&'static str> {
    Html(
        r#"
    <html>
    <body>
    <input type=file id="fileinput"/>

    <script>
// Select your input type file and store it in a variable
const input = document.getElementById('fileinput');

// This will upload the file after having read it
const upload = (file) => {
  fetch('/', { // Your POST endpoint
    method: 'POST',
    headers: {
      // Content-Type may need to be completely **omitted**
      // or you may need something
      "Content-Type": "You will perhaps need to define a content-type here"
    },
    body: file // This is your file object
  }).then(
    response => response.text() // if the response is a JSON object
  ).then(
    success => console.log(success) // Handle the success response object
  ).catch(
    error => console.log(error) // Handle the error response object
  );
};

// Event handler executed when a file is selected
const onSelectFile = () => upload(input.files[0]);

// Add a listener on your input
// It will be triggered when a file will be selected
input.addEventListener('change', onSelectFile, false);
    </script>
    </body>


    "#,
    )
}

#[post("/", data = "<data>")]
async fn file(data: Data<'_>) -> Option<String> {
    let string = data
        .open(512u32.megabytes())
        .into_string()
        .await
        .ok()?
        .into_inner();
    let mut cursor = Cursor::new(string.replace(",", "."));

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(&mut cursor);

    for result in rdr.deserialize() {
        // An error may occur, so abort the program in an unfriendly way.
        // We will make this more friendly later!
        let record: Statement = result
            .map_err(|e| {
                println!("{:?}", e);
                1
            })
            .ok()?;
        // Print a debug version of the record.
        println!("{:?}", record);
    }
    return String::from(String::new()).into();
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, file])
}
