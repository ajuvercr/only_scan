use std::{path::Path, sync::Arc};

use pulldown_cmark::{html, Options, Parser};
use rocket::tokio::fs;
use serde::{Deserialize, Serialize};

use super::{BlogError, HTML};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct FrontMatter {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    draft: bool,
}

#[derive(Debug, Serialize)]
pub struct Post {
    front: FrontMatter,
    raw: String,
    html: HTML,
}
pub type APost = Arc<Post>;

impl Post {
    pub async fn load(path: &Path) -> Result<Self, BlogError> {
        let contents = fs::read_to_string(path)
            .await
            .map_err(|e| BlogError::IO(e.kind()))?;

        // Parse frontmatter
        let mut lines = contents.lines();
        let front: FrontMatter = if lines.next().unwrap_or("") == "---" {
            let front: String = lines.take_while(|&p| p != "---").collect();
            serde_yaml::from_str(&front)?
        } else {
            FrontMatter::default()
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
