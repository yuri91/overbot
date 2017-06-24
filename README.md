# Overbot

This program is a work in progress.

Telegram Bot Manager: it abstracts away the boring stuff and leave you the fun.

Proper documentation will arrive eventually (or not).

Example .toml configuration:

```

token = "bot-token-1"

[[command]]
prefix = "/json"
executable = "./json_test.py"
input = "json"
output = "json"

[[command]]
prefix = "/free"
executable = "free"
args = ["-h"]
input = "text"
output = "textmono"

```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
