use chrono::naive::NaiveDate;
use serde::{self, de, Deserialize, Deserializer, Serializer};

use regex::Regex;

use super::Description;

const FORMAT: &'static str = "%d/%m/%Y";

pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{}", date.format(FORMAT));
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
}

pub fn deserialize_spacy_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let re = Regex::new(r"\s+").unwrap();
    let s: String = de::Deserialize::deserialize(deserializer)?;
    let st: String = re.replace_all(&s, " ").as_ref().trim().to_string();
    Ok(if st.is_empty() { None } else { Some(st) })
}

pub fn deserialize_betaling<'de, D>(deserializer: D) -> Result<Result<Description, &'static str>, D::Error>
where D: de::Deserializer<'de>,
{
    let s: String = de::Deserialize::deserialize(deserializer)?;
    Ok(s.parse())
}
