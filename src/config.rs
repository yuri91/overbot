use std::fs;
use std::io::prelude::*;
use super::toml;
use super::errors::*;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    Text,
    Json,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    Text,
    TextMono,
    Markdown,
    Html,
    Json,
}
#[derive(Clone, Deserialize)]
pub struct Bot {
    pub token: String,
    #[serde(rename = "command")]
    pub commands: Vec<Command>,
    pub allowed: Option<Vec<i64>>,
}
#[derive(Clone, Deserialize)]
pub struct Command {
    pub prefix: String,
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub input: InputType,
    pub output: OutputType,
    pub allowed: Option<Vec<i64>>,
}
impl Command {
    pub fn allowed(&self, id: i64) -> bool {
        if let Some(ref allowed) = self.allowed {
            match allowed.binary_search(&id) {
                Ok(_) => return true,
                Err(_) => return false,
            }
        } else {
            return true;
        }
    }
}
#[derive(Clone, Deserialize)]
pub struct Config {
    #[serde(rename = "bot")]
    pub bots: Vec<Bot>,
    pub allowed: Option<Vec<i64>>,
}
impl Config {
    fn apply_inheritance(&mut self) {
        for bot in &mut self.bots {
            for command in &mut bot.commands {
                if command.allowed.is_none() {
                    if bot.allowed.is_some() {
                        command.allowed = bot.allowed.clone();
                    } else if self.allowed.is_some() {
                        command.allowed = self.allowed.clone();
                    }
                }
                if let Some(ref mut allowed) = command.allowed {
                    allowed.sort();
                }
            }
        }
    }
}

pub fn get(path: &str) -> Result<Config> {
    let mut config_file = fs::File::open(path)?;
    let mut content = String::new();
    config_file.read_to_string(&mut content)?;
    let mut config: Config = toml::from_str(&content)?;
    config.apply_inheritance();
    Ok(config)
}
