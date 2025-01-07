use serde::Deserialize;
use serde::Serialize;
use serde_with_macros::skip_serializing_none;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[skip_serializing_none]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "$schema")]
    pub schema: String,
    #[serde(rename = "secondary_prompt")]
    pub secondary_prompt: SecondaryPrompt,
    pub blocks: Vec<Block>,
    pub version: i64,
    #[serde(rename = "final_space")]
    pub final_space: bool,
}

#[skip_serializing_none]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryPrompt {
    pub template: String,
    pub foreground: String,
    pub background: String,
}

#[skip_serializing_none]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    #[serde(rename = "type")]
    pub type_field: String,
    pub alignment: String,
    pub segments: Vec<Segment>,
    #[serde(rename = "min_width")]
    pub min_width: Option<i64>,
    pub newline: Option<bool>,
}

#[skip_serializing_none]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub properties: Option<Properties>,
    pub style: String,
    #[serde(rename = "leading_diamond")]
    pub leading_diamond: Option<String>,
    #[serde(rename = "trailing_diamond")]
    pub trailing_diamond: Option<String>,
    pub template: String,
    pub foreground: String,
    pub background: Option<String>,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "background_templates")]
    #[serde(default)]
    pub background_templates: Option<Vec<String>>,
    #[serde(rename = "foreground_templates")]
    #[serde(default)]
    pub foreground_templates: Option<Vec<String>>,
}

#[skip_serializing_none]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Properties {
    #[serde(rename = "folder_icon")]
    pub folder_icon: Option<String>,
    #[serde(rename = "folder_separator_icon")]
    pub folder_separator_icon: Option<String>,
    #[serde(rename = "max_depth")]
    pub max_depth: Option<i64>,
    pub style: Option<String>,
    pub threshold: Option<i64>,
    #[serde(rename = "time_format")]
    pub time_format: Option<String>,
    #[serde(rename = "fetch_version")]
    pub fetch_version: Option<bool>,
    #[serde(rename = "home_enabled")]
    pub home_enabled: Option<bool>,
    #[serde(rename = "always_enabled")]
    pub always_enabled: Option<bool>,
    pub command: Option<String>,
    pub shell: Option<String>,
    #[serde(rename = "parse_kubeconfig")]
    pub parse_kubeconfig: Option<bool>,
    #[serde(rename = "branch_max_length")]
    pub branch_max_length: Option<i64>,
    #[serde(rename = "fetch_stash_count")]
    pub fetch_stash_count: Option<bool>,
    #[serde(rename = "fetch_status")]
    pub fetch_status: Option<bool>,
    #[serde(rename = "fetch_upstream_icon")]
    pub fetch_upstream_icon: Option<bool>,
}

impl Config {
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn Error>> {
        // Open the file in read-only mode with buffer.
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `User`.
        let conf = serde_json::from_reader(reader)?;

        // Return the `Config`.
        Ok(conf)
    }

    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) {
        let file = File::create(path).expect("File path should exist");
        serde_json::to_writer_pretty(file, &self.clone()).unwrap();
    }

    pub fn update_kubectl_background_template(&mut self, vec: Vec<String>) {
        for bl in self.blocks.iter_mut() {
            for segm in bl.segments.iter_mut() {
                if segm.type_field == "kubectl" {
                    segm.background_templates = Some(vec);
                    return;
                }
            }
        }
    }

    pub fn update_kubectl_foreground_template(&mut self, vec: Vec<String>) {
        for bl in self.blocks.iter_mut() {
            for segm in bl.segments.iter_mut() {
                if segm.type_field == "kubectl" {
                    segm.foreground_templates = Some(vec);
                    return;
                }
            }
        }
    }
}
