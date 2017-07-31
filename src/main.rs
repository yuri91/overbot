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
use telegram_bot_client::{BotFactory, Update, Bot};

extern crate telegram_bot_types;
use telegram_bot_types as types;

use tokio_core::reactor;
use tokio_core::reactor::Handle;
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



fn handle_message(
    handle: Handle,
    bot: Bot,
    config: &config::Bot,
    msg: serde_json::Value,
) -> Box<Future<Item = (), Error = errors::Error>> {
    let original = msg.clone();
    let msg: types::response::Message =
        serde_json::from_value(msg).expect("Unexpected message format");
    let text = msg.text.clone().unwrap_or(String::new());
    let cmd = config
        .commands
        .iter()
        .filter(|c| {
            c.allowed(msg.from.id) && c.allowed(msg.chat.id) && c.regex.is_match(&text) &&
                c.mode == config::Mode::Message
        })
        .nth(0);
    let cmd = match cmd {
        Some(c) => c.clone(),
        None => return Box::new(future::ok(())),
    };
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
    let child = Command::new(&cmd.executable)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn_async(&handle);
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return Box::new(future::err(e.into())),
    };
    let stdin = child.stdin().take().unwrap();
    let res = match cmd.input {
        config::InputType::Text => io::write_all(stdin, text.into_bytes()),
        config::InputType::Json => io::write_all(stdin, serde_json::to_vec(&original).unwrap()),
    };
    let res = res.map_err(|e| e.into()).and_then(|_| {
        child.wait_with_output().map_err(|e| e.into()).and_then(
            move |out| {
                let mut out = String::from_utf8(out.stdout).unwrap();
                println!("out: {:?}", out);
                if cmd.output == config::OutputType::TextMono {
                    out = format!("```{}```", out);
                }
                let work;
                if cmd.output == config::OutputType::Json {
                    let json: Result<serde_json::Value, _> = serde_json::from_str(&out);
                    let json = match json {
                        Ok(j) => j,
                        Err(e) => return future::err(e.into()),
                    };
                    work = bot.request::<_, serde_json::Value>("sendMessage", &json);
                } else {
                    let parse_mode = match cmd.output {
                        config::OutputType::Text => types::request::ParseMode::Text,
                        config::OutputType::TextMono => types::request::ParseMode::Markdown,
                        config::OutputType::Markdown => types::request::ParseMode::Markdown,
                        config::OutputType::Html => types::request::ParseMode::Html,
                        config::OutputType::Json => unreachable!(),
                    };
                    work = bot.request::<_, serde_json::Value>(
                        "sendMessage",
                        &serde_json::to_value(
                            types::request::Message::new(msg.chat.id, out).parse_mode(
                                parse_mode,
                            ),
                        ).unwrap(),
                    );
                }
                let work = work.and_then(|r| {
                    println!("{:?}", r);
                    future::ok(())
                }).map_err(|e| println!("error: {:?}", e));
                handle.spawn(work);
                future::ok(())
            },
        )
    });
    Box::new(res)
}

fn handle_inline_query(
    handle: Handle,
    bot: Bot,
    config: &config::Bot,
    query: serde_json::Value,
) -> Box<Future<Item = (), Error = errors::Error>> {
    let original = query.clone();
    let query: types::response::InlineQuery =
        serde_json::from_value(query).expect("malformed message");
    let offset = query.offset.parse::<i32>().unwrap_or(0);
    let cmd = config
        .commands
        .iter()
        .filter(|c| {
            c.allowed(query.from.id) && c.regex.is_match(&query.query) &&
                c.mode == config::Mode::Inline
        })
        .nth(0);
    let cmd = match cmd {
        Some(c) => c.clone(),
        None => return Box::new(future::ok(())),
    };
    let args: Vec<String> = {
        let captures = cmd.regex.captures(&query.query).expect(
            "we already checked for match",
        );
        cmd.args
            .iter()
            .map(|a| {
                //Ugly hack part 1
                let a = a.replace("${offset}", "$${offset}");
                let mut buf = String::new();
                captures.expand(&a, &mut buf);
                //Ugly hack part 2
                buf.replace("$${offset}", "${offset}")
            })
            .collect()
    };
    let query_text = query.query.clone();
    let h = handle.clone();
    let batch_size = 10;
    let res = future::loop_fn(
        (0, Vec::new()),
        move |(iter, mut answers)| -> Box<Future<Item = future::Loop<_, _>, Error = errors::Error>> {
            if iter == batch_size - 1 {
                return Box::new(future::ok(future::Loop::Break((iter, answers))));
            }
            let args: Vec<_> = args.iter()
                .map(|a| a.replace("${offset}", &(offset + iter).to_string()))
                .collect();
            let child = Command::new(&cmd.executable)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn_async(&h);
            let mut child = match child {
                Ok(c) => c,
                Err(e) => return Box::new(future::err(e.into())),
            };
            let stdin = child.stdin().take().unwrap();
            let res = match cmd.input {
                config::InputType::Text => io::write_all(stdin, query_text.clone().into_bytes()),
                config::InputType::Json => {
                    io::write_all(stdin, serde_json::to_vec(&original).unwrap())
                }
            };
            Box::new(res.map_err(|e| e.into()).and_then(move |_| {
                child.wait_with_output().map_err(|e| e.into()).and_then(
                    move |out| {
                        let out = String::from_utf8(out.stdout).unwrap();
                        answers.push(out);
                        Ok(future::Loop::Continue((iter + 1, answers)))
                    },
                )
            }))
        },
    );
    let res = res.and_then(move |(_, mut answers)| {
        answers.retain(|a| !a.is_empty());
        let new_offset = offset + answers.len() as i32;
        let results: Vec<_> = answers
            .into_iter()
            .enumerate()
            .map(|(i, a)| {
                types::request::InlineQueryResult::article(i.to_string(), a.clone(), a.clone())
            })
            .collect();
        let answer =
            types::request::AnswerInlineQuery::new(query.id, results, new_offset.to_string());
        let work = bot.request::<_, serde_json::Value>(
            "answerInlineQuery",
            &serde_json::to_value(answer).unwrap(),
        );
        let work = work.and_then(|r| {
            println!("{:?}", r);
            future::ok(())
        }).map_err(|e| println!("error: {:?}", e));
        handle.spawn(work);
        future::ok(())
    });
    Box::new(res)
}
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
            .from_err()
            .and_then(move |update| {
                println!("{:?}", update);
                match update {
                    Update::Message(msg) => {
                        handle_message(handle.clone(), bot.clone(), &config_bot, msg)
                    }
                    Update::InlineQuery(query) => {
                        handle_inline_query(handle.clone(), bot.clone(), &config_bot, query)
                    }
                    _ => Box::new(future::ok(())),

                }
            })
            .or_else(|e| {
                println!("Error: {:?}", e);
                future::ok::<_, ()>(())
            })
            .for_each(|_| future::ok(()))
    }));
    event_loop.run(work).expect("Unexpected failure");
}
