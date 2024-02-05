use std::{collections::HashMap, str::FromStr};

use crate::repository::Repository;
use chrono::NaiveDate;
use regex::Regex;
use rocket::serde::{Deserialize, Serialize};

mod my_date;

#[derive(Deserialize, Debug, Clone)]
struct NoteUgly {
    #[serde(rename = "gestructureerde mededeling")]
    #[serde(deserialize_with = "my_date::deserialize_spacy_string")]
    structured: Option<String>,
    #[serde(rename = "Vrije mededeling")]
    #[serde(deserialize_with = "my_date::deserialize_spacy_string")]
    free: Option<String>,
}

impl NoteUgly {
    fn into(self) -> Note {
        let NoteUgly { structured, free } = self;
        Note { structured, free }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct StatementUgly {
    #[serde(default)]
    id: ID,
    #[serde(default)]
    category: Option<String>,
    #[serde(rename = "Omschrijving")]
    #[serde(deserialize_with = "my_date::deserialize_betaling")]
    description: Result<Description, String>,
    #[serde(rename = "Bedrag")]
    amount: f32,
    #[serde(rename = "Datum")]
    #[serde(with = "my_date")]
    date: NaiveDate,
    #[serde(rename = "Naam tegenpartij")]
    #[serde(deserialize_with = "my_date::deserialize_spacy_string")]
    tegenpartij: Option<String>,
    #[serde(flatten)]
    note: NoteUgly,
}

impl StatementUgly {
    pub fn into(self) -> Statement {
        let StatementUgly {
            id,
            category,
            description,
            amount,
            date,
            tegenpartij,
            note,
        } = self;
        let note = note.into();
        Statement {
            id,
            category,
            description: description.ok(),
            amount: (amount * 100.0) as isize,
            date,
            tegenpartij,
            note,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Note {
    structured: Option<String>,
    free: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
        if splits[0].contains("DOMICILIERING")
            || splits[0].contains("DOORLOPENDE")
            || splits[0].starts_with("OVERSCHRIJVING")
        {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ID(pub String);
impl Default for ID {
    fn default() -> Self {
        ID(uuid::Uuid::new_v4().to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Statement {
    pub id: ID,
    pub category: Option<String>,
    description: Option<Description>,
    amount: isize,
    #[serde(with = "my_date")]
    pub date: NaiveDate,
    tegenpartij: Option<String>,
    note: Note,
}

impl Statement {
    pub fn needs_categorised(&self) -> bool {
        self.category.is_none()
    }
    pub fn to_output<'a, 'b>(&'b self, pay: &'a str) -> ScanOutput<'a, 'b> {
        let name = self
            .description
            .as_ref()
            .map(|x| x.label.as_str())
            .or(self.tegenpartij.as_ref().map(String::as_str))
            .unwrap_or("Nothing found :(");

        ScanOutput {
            date: &self.date,
            pay,
            amount: self.amount as f32 / 100.0,
            category: self.category.as_ref().unwrap().to_string(),
            name: name.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupedStatement {
    pub key: String,
    pub statements: Vec<Statement>,
    pub category: Option<String>,
}

impl GroupedStatement {
    pub fn new(key: String) -> GroupedStatement {
        Self {
            key,
            statements: Vec::new(),
            category: None,
        }
    }

    pub fn total(&self) -> isize {
        self.statements.iter().map(|x| x.amount).sum()
    }

    pub fn delete(&mut self, id: &str) {
        self.statements.retain(|x| x.id.0 != id);
    }

    pub fn needs_categorised(&self) -> bool {
        self.category.is_none()
    }

    pub fn add_statement(&mut self, statement: &Statement) {
        self.statements.push(statement.clone());
    }

    pub fn sort(&mut self) {
        self.statements.sort_by_key(|x| x.date);
    }
}

pub type Scans = Repository<Vec<Scan>>;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Scan {
    pub id: String,
    pub grouped: Vec<GroupedStatement>,
}

#[allow(deprecated)]
impl Scan {
    pub fn new(items: Vec<Statement>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();

        let mut grouped: HashMap<String, GroupedStatement> = HashMap::new();

        items.iter().for_each(|x| {
            let key = format!(
                "{} {}",
                x.description
                    .as_ref()
                    .map(|x| x.label.as_str())
                    .unwrap_or(""),
                x.tegenpartij.as_deref().unwrap_or("")
            );
            if let Some(gs) = grouped.get_mut(&key) {
                gs.add_statement(x)
            } else {
                let mut statemens = GroupedStatement::new(key.clone());
                statemens.add_statement(x);
                grouped.insert(key, statemens);
            }
        });

        grouped.values_mut().for_each(|x| x.sort());
        let mut grouped: Vec<GroupedStatement> = grouped.into_values().collect();

        grouped.sort_by_key(|x| x.statements.iter().map(|x| x.amount).sum::<isize>());

        grouped.push(GroupedStatement::new("deleted".to_string()));

        Self { id, grouped }
    }

    pub fn delete_item(&mut self, group_id: &str, item_id: &str) {
        let mut item = None;
        if let Some(group) = self.grouped.iter_mut().filter(|x| x.key == group_id).next() {
            if let Some(x) = group.statements.iter().filter(|x| x.id.0 == item_id).next() {
                item = Some(x.clone());
            }

            group.statements.retain(|x| x.id.0 != item_id)
        }

        if let Some(y) = item {
            self.grouped.last_mut().unwrap().add_statement(&y);
        }
    }

    pub fn delete(&mut self, id: &str) {
        self.grouped.retain(|x| x.key != id);
    }

    pub fn get_first<'a>(&'a self) -> Option<&'a GroupedStatement> {
        self.grouped.iter().filter(|x| x.needs_categorised()).next()
    }

    pub fn count_done(&self) -> (usize, usize) {
        let done = self
            .grouped
            .iter()
            .filter(|x| !x.needs_categorised())
            .map(|x| x.statements.len())
            .sum();

        let total = self.grouped.iter().map(|x| x.statements.len()).sum();
        (done, total)
    }

    pub fn categorise(&mut self, uuid: &str, category: &str) {
        if let Some(item) = self.grouped.iter_mut().filter(|x| x.key == uuid).next() {
            item.statements
                .iter_mut()
                .for_each(|x| x.category = Some(category.to_string()));
            item.category = Some(category.to_string());
        }
    }
}

pub struct ScanOutput<'r, 'a> {
    date: &'a NaiveDate,
    name: String,
    pay: &'r str,
    amount: f32,
    category: String,
}

use std::fmt;
impl fmt::Display for ScanOutput<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date_str = self.date.format("%Y-%m-%d");
        writeln!(f, "{} * \"{}\"", date_str, self.name)?;
        writeln!(f, "    {} {:.2}", self.pay, self.amount)?;
        writeln!(f, "    {}", self.category)?;
        Ok(())
    }
}
