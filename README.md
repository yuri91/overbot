# Overbot - A telegram bot helper

The goal of Overbot is to remove all the boilerplate necessary to write a telegram bot,
leaving to you only the business logic to care about.

Overbot achieve this by handling the network communication, while you specify the bot behavior in a configuration file.

Supported formats are `json` and `toml`. The following is a simple example using the toml format:

```
#mybot.toml

token = "mytoken"

[[command]]
regex='hello ([A-z]+)'
executable = 'echo'
args = ['$1']
input = 'text'
output = 'text'
```

The following parameters are avalable:

## token

The token required to impersonate the bot

## command

Any number of command blocks are supported, with the following supported parameters:

### regex

The regex used to match this particular command.

The commands are tested for matching in order, and the first one to match is executed.

The supported synntax is the one supported by the rust regex crate.

It is possible to specify capture groups, and reference them later (by number and/or name) in the `args` field.

### executable

The absolute path of a program to execute in response to the received message.

The program will receive the message via `stdin`, and its `stdout` will be the body of the response message.

### args

An array of arguments to pass to the executable.

The `$` symbol can be used as a prefix to a capture group number or name, that will be expanded accordingly to the provided query.

The capture group `$0` is the entire match.

### input

The input mode that will be used. Supported modes are:

- **json**: the raw json message will be sent to stdin
- **text**: the `text` field of the message will be sent to stdio

### input

The output mode that will be used. Supported modes are:

- **json**: the program stdout is expected to contain a raw json message response
- **text**: the program stdout is expected to contain the text field of the message response
- **textmono**: the program stdout is expected to contain the text field of the message response, that will be enclosed in a `monopaced` formatting
- **markdown**: the program stdout is expected to contain the text field of the message response, and the subset of Markdown accepted by telegram is recognized
- **html**: the program stdout is expected to contain the text field of the message response, and the subset of HTML accepted by telegram is recognized

## Access control

Optionally, it is possible to specify a `allowed` parameter, as a list of telegram ids that are accepted for the bot, or for the single command if the field is present in a `[[command]]` section.

The ids may be both group ids or user ids.

## Run the program

The program accept as a parameter a folder containing any number of `.toml` or `.json` configuration files (one per bot):

```
overbot my_bot_dir
```

## Limitations

The program is currently a work in progress, and lacks many desirable features, in particular:

- [ ] Decent and human readable error handling for configuration errors
- [ ] Inline mode support
- [ ] Decent and configurable logging

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
