use std::str::FromStr;

use crate::repository::Repository;
use chrono::NaiveDate;
use diesel::dsl::Desc;
use regex::Regex;
use rocket::serde::{Deserialize, Serialize};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

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

#[derive(Serialize, Debug)]
pub struct Dom {
    way: String,
    label: String,
}

#[derive(Serialize, Debug)]
pub struct Bet {
    way: String,
    label: String,
    user: String,
}

#[derive(Serialize, Debug)]
pub struct Description {
    way: String,
    label: String,
    user: Option<String>,
}

impl FromStr for Description {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"[0-9]{2}-[0-9]{2}").unwrap();
        let splits = re.splitn(s, 2).collect::<Vec<_>>();

        if splits.len() < 2 {
            return Err("Not enough splits!");
        }

        let way = splits[0].trim().to_string();
        let label;
        let mut user = None;
        // Dom
        if splits[0].contains("DOMICILIERING") || splits[0].contains("DOORLOPENDE") || splits[0].starts_with("OVERSCHRIJVING") {
            label = s
                .split(':')
                .nth(1)
                .ok_or("No ':' in DOMICILIERING")?
                .trim()
                .to_string();
        } else {
            let (_, s) = splits[1].split_once("UUR").ok_or("NO 'UUR' in betaling")?;
            let (l, _) = s.split_once("MET").ok_or("NO 'MET' in betaling")?;
            label = l.trim().to_string();
            user = Some(
                s.split(':')
                    .last()
                    .ok_or("NO ':' in betaling")?
                    .trim()
                    .to_string(),
            );
        }
        Ok(Description { way, label, user })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Statement {
    #[serde(rename = "Omschrijving")]
    #[serde(deserialize_with = "my_date::deserialize_betaling")]
    description: Result<Description, &'static str>,
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

pub type Scans = Repository<Vec<Scan>>;
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Scan {
    pub id: String,
    pub date: time::Date,
    pub items: Vec<ScanItem>,
}

#[allow(deprecated)]
impl Scan {
    pub fn delete(&mut self, id: &str) {
        self.items.retain(|x| x.id != id);
    }

    pub fn new_with(count: usize) -> Self {
        let mut rng = thread_rng();
        let items = (0..count)
            .map(|_| {
                ScanItem::new::<String>(
                    // String:
                    (&mut rng)
                        .sample_iter(Alphanumeric)
                        .take(7)
                        .map(char::from)
                        .collect(),
                    rng.gen_range(50..1000),
                )
            })
            .collect();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            date: time::Date::today(),
            items,
        }
    }

    pub fn get_first<'a>(&'a self) -> Option<&'a ScanItem> {
        self.items.iter().filter(|x| x.needs_categorised()).next()
    }

    pub fn count_done(&self) -> (usize, usize) {
        let done = self.items.iter().filter(|x| !x.needs_categorised()).count();
        (done, self.items.len())
    }

    pub fn categorise(&mut self, uuid: &str, name: &str, price: usize, category: &str) {
        if let Some(item) = self.items.iter_mut().filter(|x| x.id == uuid).next() {
            item.category = Some(category.to_string());
            item.name = name.to_string();
            item.price = price;
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ScanItem {
    pub id: String,
    pub name: String,
    pub price: usize,
    pub category: Option<String>,
}

impl ScanItem {
    pub fn new<S: Into<String>>(name: S, price: usize) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            price,
            category: None,
        }
    }

    pub fn needs_categorised(&self) -> bool {
        self.category.is_none()
    }
}
