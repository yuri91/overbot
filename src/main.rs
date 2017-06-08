extern crate futures;
extern crate tokio_core;
extern crate tokio_io;

extern crate tokio_process;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;

extern crate telegram_bot;
use telegram_bot::{BotFactory, Update};
use telegram_bot::errors::Error;

extern crate telegram_bot_api;
use telegram_bot_api as api;

use tokio_core::reactor;
use tokio_io::io;
use futures::{Future, Stream};
use futures::future;

use std::process::{Command,Stdio};
use tokio_process::CommandExt;

use std::fs;
use std::io::prelude::*;

mod config {
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
        pub command: Vec<Command>
    }
    #[derive(Clone, Deserialize)]
    pub struct Command {
        pub prefix: String,
        pub executable: String,
        #[serde(default)]
        pub args: Vec<String>,
        pub input: InputType,
        pub output: OutputType,
    }
    #[derive(Clone, Deserialize)]
    pub struct Config {
        pub bot: Vec<Bot>
    }
}

fn main() {
    let mut config_file = fs::File::open("bot.toml").expect("no file");
    let mut content = String::new();
    config_file.read_to_string(&mut content).expect("can't read file");
    let config: config::Config = toml::from_str(&content).expect("wrong config");

    let mut event_loop = reactor::Core::new().unwrap();
    let handle = event_loop.handle();

    let factory = BotFactory::new(handle.clone());
    let work = future::join_all(config.bot.into_iter().map(|config_bot| {
        println!("{}",config_bot.token);
        let handle = handle.clone();
        let (bot,updates) = factory.new_bot(&config_bot.token);
        updates.filter_map(move|update| {
            println!("{:?}", update);
            match update {
                Update::Message(msg) => {
                    let original = msg.clone();
                    let msg: api::response::Message = serde_json::from_value(msg)
                        .expect("Unexpected message format");
                    if let Some(text) = msg.text.clone() {
                        for c in &config_bot.command {
                            if text.starts_with(&c.prefix) {
                                return Some((c.clone(),msg,original))
                            }
                        }
                    }
                },
                _ => {}
            };
            None
        })
        .for_each(move|(cmd,msg,original)| {
            let handle = handle.clone();
            let bot = bot.clone();
            let mut child = Command::new(&cmd.executable)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn_async(&handle).expect("failed to spawn process");
            let stdin = child.stdin().take().unwrap();
            let res = match cmd.input {
                config::InputType::Text => io::write_all(stdin,msg.text.clone().unwrap().into_bytes()),
                config::InputType::Json => io::write_all(stdin,serde_json::to_vec(&original).unwrap()),
            };
            let res = res.map_err(|e|e.into());
            res.and_then(|_| child.wait_with_output().map_err(|e|Error::from(e)).and_then(move |out| {
                let mut out = String::from_utf8(out.stdout).unwrap();
                println!("out: {:?}",out);
                if cmd.output == config::OutputType::TextMono {
                    out = format!("```{}```",out);
                }
                let work;
                if cmd.output == config::OutputType::Json {
                    let json : serde_json::Value = serde_json::from_str(&out)?;
                    work = bot
                        .request::<_,serde_json::Value>("sendMessage",&json);
                } else {
                    let parse_mode = match cmd.output {
                        config::OutputType::Text => api::request::ParseMode::Text,
                        config::OutputType::TextMono => api::request::ParseMode::Markdown,
                        config::OutputType::Markdown => api::request::ParseMode::Markdown,
                        config::OutputType::Html => api::request::ParseMode::Html,
                        config::OutputType::Json => unreachable!()
                    };
                    work = bot
                        .request::<_,serde_json::Value>("sendMessage",
                                 &serde_json::to_value(api::request::Message::new(msg.chat.id,out).parse_mode(parse_mode))
                                 .unwrap());
                }
                let work = work.and_then(|r| {
                        println!("{:?}", r);
                        future::ok(())
                    }).map_err(|e| println!("error: {:?}",e));
                handle.spawn(work);
                Ok(())
            })).or_else(|e|{
                println!("error: {:?}",e);
                future::ok(())
            })
        })
    }));
    event_loop.run(work).expect("exit with error");
}
