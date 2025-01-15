use std::env;

use toml::Table;

#[derive(Debug)]
pub struct ParsedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub paths: Vec<String>,
}

pub struct CommandParser {
    metadata: Table,
}

impl CommandParser {
    pub fn new() -> Self {
        let metadata = toml::from_str(include_str!("./meta.toml")).unwrap();
        CommandParser { metadata }
    }

    pub fn parse(&self, command: &str) -> ParsedCommand {
        let args = self.split_command_line(&command);
        let mut iterable = args.iter();
        let command = iterable.next().map_or("", |v| &v).to_string();
        let mut args = iterable
            .take(args.len() - 1)
            .map(|f| f.clone())
            .collect::<Vec<_>>();
        args.iter_mut().for_each(|f| {
            if f.starts_with("~") {
                *f = self.parse_path(f).join("/");
            }
        });
        let path = args.last().map_or("", |f| f).to_owned();
        let paths = self.parse_path(&path);
        let meta = self.metadata.get(
            command
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("_")
                .as_str(),
        );

        if !path.is_empty() && meta.is_some() && meta.unwrap().get("expects").is_some() {
            match args.last_mut() {
                Some(arg) => *arg = paths.join("/"),
                None => todo!(),
            }
        }

        ParsedCommand {
            command,
            args,
            paths,
        }
    }

    fn split_command_line(&self, input: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_type: Option<char> = None;

        for c in input.chars() {
            match c {
                '"' | '\'' => {
                    if in_quotes && quote_type == Some(c) {
                        in_quotes = false;
                        quote_type = None;
                    } else if !in_quotes {
                        in_quotes = true;
                        quote_type = Some(c);
                    } else {
                        current.push(c);
                    }
                }
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        args.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            args.push(current);
        }

        args
    }

    fn parse_path(&self, input: &str) -> Vec<String> {
        let mut input = input.to_string();
        let userpath = &format!(
            "/home/{}/",
            env::var("USER").unwrap_or_else(|_| "Unknown".to_string())
        );

        let home_indicators = ["~/", "~"];

        for indicator in home_indicators {
            input = input.replace(indicator, userpath);
        }

        if !input.starts_with("/") {
            input = format!("./{}", input);
        }

        return input.split("/").map(|f| f.to_string()).collect::<Vec<_>>();
    }
}
