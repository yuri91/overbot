use std;
use std::fs;
use std::fmt;
use std::path::Path;
use std::io::prelude::*;
use super::toml;
use super::serde::de;
use super::serde_json;
use super::regex;
use super::errors::{Result, ResultExt, ErrorKind};

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
type RegexResult<E> = std::result::Result<regex::Regex, E>;
fn deserialize_regex<'de, D>(deserializer: D) -> RegexResult<D::Error>
where
    D: de::Deserializer<'de>,
{
    struct RegexDe;

    impl<'de> de::Visitor<'de> for RegexDe {
        type Value = regex::Regex;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("valid regex")
        }

        fn visit_str<E>(self, value: &str) -> RegexResult<E>
        where
            E: de::Error,
        {
            regex::Regex::new(value).map_err(|_| {
                de::Error::invalid_value(de::Unexpected::Str(value), &self)
            })
        }
    }
    deserializer.deserialize_str(RegexDe)
}
#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Message,
    Inline,
}
impl Mode {
    pub fn message() -> Mode {
        Mode::Message
    }
}
#[derive(Clone, Deserialize)]
pub struct Command {
    #[serde(deserialize_with = "deserialize_regex")]
    pub regex: regex::Regex,
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub input: InputType,
    pub output: OutputType,
    #[serde(default = "Mode::message")]
    pub mode: Mode,
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
impl Bot {
    fn apply_inheritance(&mut self) {
        for command in &mut self.commands {
            if command.allowed.is_none() && self.allowed.is_some() {
                command.allowed = self.allowed.clone();
            }
            if let Some(ref mut allowed) = command.allowed {
                allowed.sort();
            }

        }
    }
    fn fix_relative_paths<P: AsRef<Path>>(&mut self, base_dir: P) {
        let base_dir = base_dir.as_ref();
        for command in &mut self.commands {
            let abs = base_dir.join(&command.executable);
            command.executable = abs.into_os_string().into_string().expect(
                "I expected this string to be valid UTF-8",
            )
        }
    }
}

enum ConfType {
    Json,
    Toml,
}
fn get<P: AsRef<Path>>(path: P, ty: ConfType) -> Result<Bot> {
    let path = path.as_ref();
    let mut config_file = fs::File::open(path).chain_err(|| {
        ErrorKind::Config(path.to_string_lossy().into_owned(), "Cannot open")
    })?;
    let mut content = String::new();
    config_file.read_to_string(&mut content).chain_err(|| {
        ErrorKind::Config(path.to_string_lossy().into_owned(), "Error in reading file")
    })?;
    let mut bot: Bot = match ty {
        ConfType::Toml => {
            toml::from_str(&content).chain_err(|| {
                ErrorKind::Config(path.to_string_lossy().into_owned(), "Error in conf file")
            })?
        }
        ConfType::Json => {
            serde_json::from_str(&content).chain_err(|| {
                ErrorKind::Config(path.to_string_lossy().into_owned(), "Error in conf file")
            })?
        }

    };
    bot.apply_inheritance();
    bot.fix_relative_paths(path.parent().expect("This should be a file"));
    Ok(bot)
}

pub fn get_all<P: AsRef<Path>>(dir: P) -> Result<Vec<Bot>> {
    let dir = dir.as_ref();
    let entries = fs::read_dir(dir).chain_err(|| {
        ErrorKind::Config(dir.to_string_lossy().into_owned(), "Conf dir not found")
    })?;
    entries
        .filter_map(|e| {
            if let Ok(entry) = e {
                let p = entry.path();
                if let Some(ext) = p.extension() {
                    if ext == "toml" {
                        return Some(get(p.to_str().unwrap(), ConfType::Toml));
                    } else if ext == "json" {
                        return Some(get(p.to_str().unwrap(), ConfType::Json));
                    }
                }
            }
            None
        })
        .collect()
}
