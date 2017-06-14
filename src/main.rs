extern crate futures;
extern crate tokio_core;
extern crate tokio_io;

extern crate tokio_process;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;

#[macro_use]
extern crate error_chain;

extern crate telegram_bot_client;
use telegram_bot_client::{BotFactory, Update};
use telegram_bot_client::errors::Error as TGError;

extern crate telegram_bot_types;
use telegram_bot_types as types;

use tokio_core::reactor;
use tokio_io::io;
use futures::{Future, Stream};
use futures::future;

use std::process::{Command,Stdio};
use tokio_process::CommandExt;

mod config;
mod errors;


fn main() {
    let config = config::get("test.toml").expect("config error");

    let mut event_loop = reactor::Core::new().unwrap();
    let handle = event_loop.handle();

    let factory = BotFactory::new(handle.clone());
    let work = future::join_all(config.bots.into_iter().map(|config_bot| {
        let handle = handle.clone();
        let (bot,updates) = factory.new_bot(&config_bot.token);
        updates.filter_map(move|update| {
            println!("{:?}", update);
            match update {
                Update::Message(msg) => {
                    let original = msg.clone();
                    let msg: types::response::Message = serde_json::from_value(msg)
                        .expect("Unexpected message format");
                    if let Some(text) = msg.text.clone() {
                        for c in &config_bot.commands {
                            if text.starts_with(&c.prefix) &&
                                c.allowed(msg.from.id) &&
                                c.allowed(msg.chat.id) {
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
            res.and_then(|_| child.wait_with_output().map_err(|e|TGError::from(e)).and_then(move |out| {
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
                        config::OutputType::Text => types::request::ParseMode::Text,
                        config::OutputType::TextMono => types::request::ParseMode::Markdown,
                        config::OutputType::Markdown => types::request::ParseMode::Markdown,
                        config::OutputType::Html => types::request::ParseMode::Html,
                        config::OutputType::Json => unreachable!()
                    };
                    work = bot
                        .request::<_,serde_json::Value>("sendMessage",
                                 &serde_json::to_value(types::request::Message::new(msg.chat.id,out).parse_mode(parse_mode))
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
