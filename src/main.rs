extern crate futures;
extern crate tokio_core;
extern crate tokio_io;

extern crate tokio_process;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;

extern crate regex;

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

use std::process::{Command, Stdio};
use tokio_process::CommandExt;

extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "overbot", about = "A telegram bot manager")]
struct Opt {
    #[structopt(help = "Config files directory")]
    config_dir: String,
}

mod config;
mod errors;


fn main() {
    let opt = Opt::from_args();
    let config = config::get_all(&opt.config_dir).expect("Wrong path for config dir");

    let mut event_loop = reactor::Core::new().unwrap();
    let handle = event_loop.handle();

    let factory = BotFactory::new(handle.clone());
    let work = future::join_all(config.into_iter().map(|config_bot| {
        let handle = handle.clone();
        let (bot, updates) = factory.new_bot(&config_bot.token);
        updates
            .filter_map(move |update| {
                println!("{:?}", update);
                match update {
                    Update::Message(msg) => {
                        let original = msg.clone();
                        let msg: types::response::Message =
                            serde_json::from_value(msg).expect("Unexpected message format");
                        if let Some(text) = msg.text.clone() {
                            for c in &config_bot.commands {
                                if c.allowed(msg.from.id) && c.allowed(msg.chat.id) &&
                                    c.regex.is_match(&text)
                                {
                                    return Some((c.clone(), msg, original));
                                }
                            }
                        }
                    }
                    _ => {}
                };
                None
            })
            .for_each(move |(cmd, msg, original)| {
                let handle = handle.clone();
                let bot = bot.clone();
                let text = msg.text.clone().unwrap();
                let args: Vec<String> = {
                    let captures = cmd.regex.captures(&text).expect(
                        "we already checked for match",
                    );
                    cmd.args
                        .iter()
                        .map(|a| {
                            let mut buf = String::new();
                            captures.expand(a, &mut buf);
                            buf
                        })
                        .collect()
                };
                let mut child = Command::new(&cmd.executable)
                    .args(&args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn_async(&handle)
                    .expect("failed to spawn process");
                let stdin = child.stdin().take().unwrap();
                let res = match cmd.input {
                    config::InputType::Text => io::write_all(stdin, text.into_bytes()),
                    config::InputType::Json => {
                        io::write_all(stdin, serde_json::to_vec(&original).unwrap())
                    }
                };
                let res = res.map_err(|e| e.into());
                res.and_then(|_| {
                    child
                        .wait_with_output()
                        .map_err(|e| TGError::from(e))
                        .and_then(move |out| {
                            let mut out = String::from_utf8(out.stdout).unwrap();
                            println!("out: {:?}", out);
                            if cmd.output == config::OutputType::TextMono {
                                out = format!("```{}```", out);
                            }
                            let work;
                            if cmd.output == config::OutputType::Json {
                                let json: serde_json::Value = serde_json::from_str(&out)?;
                                work = bot.request::<_, serde_json::Value>("sendMessage", &json);
                            } else {
                                let parse_mode = match cmd.output {
                                    config::OutputType::Text => types::request::ParseMode::Text,
                                    config::OutputType::TextMono => {
                                        types::request::ParseMode::Markdown
                                    }
                                    config::OutputType::Markdown => {
                                        types::request::ParseMode::Markdown
                                    }
                                    config::OutputType::Html => types::request::ParseMode::Html,
                                    config::OutputType::Json => unreachable!(),
                                };
                                work = bot.request::<_, serde_json::Value>(
                                    "sendMessage",
                                    &serde_json::to_value(
                                        types::request::Message::new(msg.chat.id, out)
                                            .parse_mode(parse_mode),
                                    ).unwrap(),
                                );
                            }
                            let work = work.and_then(|r| {
                                println!("{:?}", r);
                                future::ok(())
                            }).map_err(|e| println!("error: {:?}", e));
                            handle.spawn(work);
                            Ok(())
                        })
                }).or_else(|e| {
                        println!("error: {:?}", e);
                        future::ok(())
                    })
            })
    }));
    event_loop.run(work).expect("exit with error");
}
