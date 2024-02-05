use std::{path::Path, sync::Arc, time::SystemTime};

use chrono::{DateTime, NaiveDate, Utc};
use pulldown_cmark::{html, Options, Parser};
use rocket::tokio::fs;
use serde::{Deserialize, Serialize};

use super::{BlogError, HTML};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FrontMatter {
    title: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    short: String,
    pub date: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct Post {
    front: FrontMatter,
    raw: String,
    html: HTML,
}
pub type APost = Arc<Post>;

#[derive(Debug, Serialize)]
pub struct PostShort {
    front: FrontMatter,
    link: String,
    pub date: NaiveDate,
    pub time_str: String,
}

impl PostShort {
    pub fn new(post: &Post, link: impl Into<String>) -> Self {
        Self {
            front: post.front.clone(),
            date: post.front.date.clone(),
            time_str: post.front.date.format("%d-%m-%Y").to_string(),
            link: link.into(),
        }
    }
}

impl Post {
    pub async fn load(path: &Path) -> Result<Self, BlogError> {
        let contents = fs::read_to_string(path)
            .await
            .map_err(|e| BlogError::IO(e.kind()))?;

        // Parse frontmatter
        let mut lines = contents.lines();
        let front: FrontMatter = if lines.next().unwrap_or("") == "---" {
            let front: String = lines
                .take_while(|&p| p != "---")
                .flat_map(|x| ["\n", x])
                .skip(1)
                .collect();
            match serde_yaml::from_str(&front) {
                Ok(x) => x,
                Err(e) => return Err(BlogError::Front(e, path.to_string_lossy().to_string())),
            }
        } else {
            return Err(BlogError::NoFrontmatter);
        };

        let start = contents
            .match_indices("\n---\n")
            .next()
            .map(|(x, _)| x + 4)
            .unwrap_or(0);

        let parser = Parser::new_ext(&contents[start..], Options::all());
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        Ok(Post {
            front,
            raw: contents,
            html: html_output,
        })
    }
}
