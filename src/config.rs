use std::fs;
use std::collections::HashSet;
use std::io::prelude::*;
use super::toml;
use super::errors::*;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum InputType {
    Text,
    Json
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum OutputType {
    Text,
    TextMono,
    Markdown,
    Html,
    Json
}
#[derive(Clone, Deserialize)]
pub struct Bot {
    pub token: String,
    #[serde(rename="command")]
    pub commands: Vec<Command>,
    #[serde(default)]
    pub whitelist: HashSet<String>,
    #[serde(default)]
    pub blacklist: HashSet<String>,
}
#[derive(Clone, Deserialize)]
pub struct Command {
    pub prefix: String,
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub input: InputType,
    pub output: OutputType,
    #[serde(default)]
    pub whitelist: HashSet<String>,
    #[serde(default)]
    pub blacklist: HashSet<String>,
}
#[derive(Clone, Deserialize)]
pub struct Config {
    #[serde(rename="bot")]
    pub bots: Vec<Bot>,
    #[serde(default)]
    pub whitelist: HashSet<String>,
    #[serde(default)]
    pub blacklist: HashSet<String>,
}
impl Config {
    fn apply_acl(&mut self) {
        for bot in &mut self.bots {
            for command in &mut bot.commands {
                if command.whitelist.len() == 0 {
                    if bot.whitelist.len() == 0 {
                        command.whitelist = self.whitelist.clone();
                    } else {
                        command.whitelist = bot.whitelist.clone();
                    }
                }
                if command.blacklist.len() == 0 {
                    if bot.blacklist.len() == 0 {
                        command.blacklist = self.blacklist.clone();
                    } else {
                        command.blacklist = bot.blacklist.clone();
                    }
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
    config.apply_acl();
    Ok(config)
}
