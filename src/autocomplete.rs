use crossterm::terminal;

use crate::parser::CommandParser;

use std::error::Error;
use std::fs::{self};
use std::io::{self};
use std::path::PathBuf;

pub struct Suggestion {
    file_name: String,
    is_dir: bool,
}

pub struct AutoComplete {}

impl AutoComplete {
    pub fn new() -> Self {
        return AutoComplete {};
    }

    pub fn autocomplete(
        &self,
        command: &str,
        parser: &CommandParser,
    ) -> Result<String, Box<dyn Error>> {
        let mut new_value = String::from(command);
        let parsed_command = parser.parse(command);
        let searched_file = parsed_command.paths.last().map_or("", |s| s.as_str());
        let in_path =
            parsed_command.paths[..parsed_command.paths.len().saturating_sub(1)].join("/");

        let mut entries = fs::read_dir(&in_path)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;
        entries.sort();

        if parsed_command.command == "cd" {
            entries = entries.into_iter().filter(|f| f.is_dir()).collect();
        }

        let terminal_width = terminal::size()?.0 as usize;

        let mut matching_file_names: Vec<Suggestion> = vec![];

        for (_i, entry) in entries.iter().enumerate() {
            let file_name = entry.file_name().unwrap().to_string_lossy().to_string();
            if searched_file.len() == 0 || file_name.starts_with(&searched_file) {
                matching_file_names.push(Suggestion {
                    file_name: file_name.clone(),
                    is_dir: entry.is_dir(),
                });
            }
        }

        if matching_file_names.len() > 1 {
            let longest_match = self.get_longest_match(&matching_file_names, searched_file);

            if longest_match.len() > searched_file.len() {
                new_value = command.replace(&searched_file, &format!("{}", longest_match));
            } else {
                let max_width = entries
                    .iter()
                    .map(|entry| entry.file_name().unwrap().to_string_lossy().len())
                    .max()
                    .unwrap_or(0);
                let columns = terminal_width / (max_width + 2); // Add 4 for padding
                println!("");

                for (i, suggestion) in matching_file_names.iter().enumerate() {
                    print!("{:<width$}", suggestion.file_name, width = max_width);
                    if (i + 1) % columns == 0 {
                        println!();
                    }
                }

                // Ensure we end with a new line
                if entries.len() % columns != 0 {
                    println!();
                }
            }
        } else if matching_file_names.len() == 1 {
            let matched = matching_file_names.first().unwrap();
            new_value = command.replace(
                &searched_file,
                &format!(
                    "{}{}",
                    matched.file_name,
                    if matched.is_dir { "/" } else { "" }
                ),
            );
        }
        Ok(new_value)
    }

    fn get_longest_match(&self, entries: &Vec<Suggestion>, search: &str) -> String {
        let mut longest_match = String::from(search);
        let mut len = longest_match.len();
        let first_entry = entries.first().unwrap().to_owned();
        loop {
            len += 1;
            let mut success = true;
            let (trying_match, _) = first_entry.file_name.split_at(len);
            for entry in entries {
                if !entry.file_name.starts_with(trying_match) {
                    success = false;
                    break;
                }
            }
            if success {
                longest_match = String::from(trying_match);
            } else {
                break;
            }
        }
        longest_match
    }
}
