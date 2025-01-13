use std::{collections::vec_deque, env, error::Error};

#[derive(Debug)]
pub struct ParsedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub paths: Vec<String>,
}

pub fn parse(command: &str) -> ParsedCommand {
    let args = split_command_line(&command);
    let mut iterable = args.iter();
    let command = iterable.next().map_or("", |v| &v).to_string();
    let args = iterable
        .take(args.len() - 1)
        .map(|f| f.clone())
        .collect::<Vec<_>>();
    let path = args.last().map_or("", |f| f).to_owned();

    ParsedCommand {
        command,
        args,
        paths: parse_path(path),
    }
}

fn split_command_line(input: &str) -> Vec<String> {
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

fn parse_path(input: String) -> Vec<String> {
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
        input = format!("/{}", input);
    }

    return input.split("/").map(|f| f.to_string()).collect::<Vec<_>>();
}
