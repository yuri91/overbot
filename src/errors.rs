use super::serde_json;
use super::toml;
use std::io;
use super::telegram_bot_client;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
        Bot(telegram_bot_client::errors::Error, telegram_bot_client::errors::ErrorKind);
    }

    foreign_links {
        Json(serde_json::Error);
        Io(io::Error);
        Toml(toml::de::Error);
    }

    errors {
    }
}
