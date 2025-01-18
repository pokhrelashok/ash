use crossterm::{
    cursor::{self, MoveLeft, MoveRight, MoveTo},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, BufRead, BufReader, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::{env, error::Error};
use std::{fs::File, io::stdout};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    about::print_about, autocomplete::AutoComplete, history::History, parser::CommandParser,
    suggestion::get_command_suggestion,
};

pub struct Shell {
    input: String,
    temp_input: String,
    history: History,
    stdout: Stdout,
    autocompleter: AutoComplete,
    parser: CommandParser,
    prompt_length: u16,
    suggestions: Vec<String>,
    suggestion_index: u8,
}

impl Drop for Shell {
    fn drop(&mut self) {
        disable_raw_mode().unwrap();
    }
}

impl Shell {
    pub fn new() -> io::Result<Self> {
        let history = History::new(format!(
            "/home/{}/.ash_history",
            env::var("USER").unwrap_or_else(|_| "Unknown".to_string())
        ))?;
        Ok(Shell {
            autocompleter: AutoComplete::new(),
            stdout: stdout(),
            input: "".to_string(),
            temp_input: "".to_string(),
            history,
            prompt_length: 0,
            suggestions: vec![],
            suggestion_index: 0,
            parser: CommandParser::new(),
        })
    }

    pub fn init(&mut self) {
        loop {
            self.input.clear();
            if let Err(e) = self.collect_input() {
                eprintln!("Error collecting input: {}", e);
                continue;
            }

            if self.input.trim() == "exit" {
                break;
            }

            if let Err(e) = self.process_input() {
                eprintln!("Error processing input: {}", e);
            }
            self.reset_states();
        }
    }

    fn collect_input(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut index: i8 = -1;
        self.print_prompt();

        loop {
            if let Ok(true) = event::poll(std::time::Duration::from_millis(500)) {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                        && key_event.code == KeyCode::Char('c')
                    {
                        self.reset_states();
                        index = -1;
                        print!("\n");
                        self.print_prompt();
                        continue;
                    }
                    match key_event.code {
                        KeyCode::Char(c) => self.handle_char_input(c)?,
                        KeyCode::Backspace => self.handle_backspace()?,
                        KeyCode::Enter => {
                            disable_raw_mode()?;
                            self.handle_enter();
                            return Ok(());
                        }
                        KeyCode::Up => {
                            if self.suggestions.len() > 0 {
                                if self.suggestion_index < self.suggestions.len() as u8 {
                                    self.suggestion_index += 1;
                                    self.print_prompt();
                                }
                                continue;
                            }

                            if self.history.count() > 0 && index < (self.history.count() - 1) as i8
                            {
                                if index == -1 {
                                    self.temp_input = self.input.clone();
                                }

                                index += 1;
                                if self.history.count() >= 10
                                    && index as usize == self.history.count() - 2
                                {
                                    self.history.fetch_more();
                                }
                                self.handle_arrow(index as usize)?;
                            }
                        }
                        KeyCode::Down => {
                            if self.suggestions.len() > 0 && self.suggestion_index > 0 {
                                self.suggestion_index -= 1;
                                self.print_prompt();
                                continue;
                            }
                            if index < 0 {
                                continue;
                            }
                            if index > 0 {
                                index -= 1;
                                self.handle_arrow(index as usize)?;
                            } else {
                                index = -1;
                                self.input = self.temp_input.clone();
                                self.print_prompt();
                            }
                        }
                        KeyCode::Tab => {
                            if !self.input.is_empty() {
                                self.autocomplete()?
                            };
                        }
                        KeyCode::Left => {
                            let (x, _) = cursor::position().unwrap();
                            if x <= self.prompt_length {
                                continue;
                            }
                            execute!(self.stdout, MoveLeft(1)).unwrap();
                        }
                        KeyCode::Right => {
                            let (x, _) = cursor::position().unwrap();
                            if x > self.prompt_length - 1 + self.input.len() as u16 {
                                if !self.suggestions.is_empty() {
                                    self.input = format!(
                                        "{}{}",
                                        self.input,
                                        self.suggestions
                                            .get(self.suggestion_index as usize)
                                            .map_or("", |x| x)
                                            .replacen(&self.input, "", 1)
                                    );
                                    self.print_prompt();
                                    continue;
                                } else {
                                    continue;
                                }
                            }

                            execute!(self.stdout, MoveRight(1)).unwrap();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn autocomplete(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        match self
            .autocompleter
            .autocomplete(self.input.as_str(), &self.parser)
        {
            Ok(new_command) => {
                self.input = new_command;
                self.print_prompt();
            }
            Err(_) => todo!(),
        }
        enable_raw_mode()?;
        Ok(())
    }

    fn print_prompt(&mut self) {
        let cwd = env::current_dir()
            .unwrap_or_default()
            .into_os_string()
            .into_string()
            .unwrap_or("".to_string());
        let wdir = cwd.split("/").last().unwrap_or_default();
        let prompt = format!("{}{}  ", "  ", wdir);
        self.prompt_length = prompt.graphemes(true).count() as u16;
        execute!(self.stdout, cursor::Hide).unwrap();
        print!("\r\x1b[2K\x1b[34m{}\x1b[0m{}", prompt, self.input);
        if self.input.len() > 0 {
            print!(
                "\x1b[2m{}\x1b[0m",
                self.suggestions
                    .get(self.suggestion_index as usize)
                    .map_or("", |x| x)
                    .replacen(&self.input, "", 1)
            );
        }
        let (_, y) = cursor::position().unwrap();
        execute!(
            self.stdout,
            MoveTo(self.prompt_length + self.input.len() as u16, y)
        )
        .unwrap();
        execute!(self.stdout, cursor::Show).unwrap();
        io::stdout().flush().unwrap();
    }

    fn handle_char_input(&mut self, c: char) -> Result<(), Box<dyn Error>> {
        let (x, y) = cursor::position().unwrap();
        self.input.insert((x - self.prompt_length) as usize, c);
        if self.input.len() > 0 {
            self.suggestions = get_command_suggestion(&self.history.commands, &self.input)
        }
        self.print_prompt();
        execute!(self.stdout, MoveTo(x + 1, y)).unwrap();
        Ok(())
    }

    fn handle_backspace(&mut self) -> Result<(), Box<dyn Error>> {
        if self.input.len() == 0 {
            return Ok(());
        }
        let (x, y) = cursor::position().unwrap();
        let pos = (x - self.prompt_length) as usize;
        if pos > 0 {
            self.input.remove(pos - 1);
            if self.input.len() > 0 {
                self.suggestions = get_command_suggestion(&self.history.commands, &self.input)
            }
            self.print_prompt();
            execute!(self.stdout, MoveTo(if x > 0 { x - 1 } else { x }, y)).unwrap();
        }
        Ok(())
    }

    fn handle_enter(&mut self) {
        println!();
        if !self.input.trim().is_empty() {
            self.history.add_command(&self.input);
        }
    }

    fn handle_arrow(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        if index < self.history.count() {
            self.input = self
                .history
                .get_command(index)
                .map_or("", |f| f)
                .to_string();
            self.print_prompt();
        }
        Ok(())
    }

    fn process_input(&mut self) -> Result<(), Box<dyn Error>> {
        let input = self.input.clone();
        let mut commands = input.split(" | ").peekable();
        let mut previous_command: Option<Child> = None;

        while let Some(command_group) = commands.next() {
            let mut split_commands = command_group.split(" && ").peekable();

            while let Some(command) = split_commands.next() {
                // Execute the current command
                let mut current_command = self.execute_command(
                    command.trim(),
                    previous_command.take(),
                    commands.peek().is_some(),
                )?;

                // If there are more commands after &&, check the success of the previous one
                if split_commands.peek().is_some() {
                    if let Some(ref mut child) = current_command {
                        let status = child.wait()?;
                        if !status.success() {
                            // If the current command fails, stop processing this group
                            break;
                        }
                    }
                }

                // Update previous_command for the next iteration
                previous_command = current_command;
            }
        }

        // Wait for the last command in the pipeline to finish
        if let Some(mut final_command) = previous_command {
            final_command.wait()?;
        }

        Ok(())
    }

    fn reset_states(&mut self) {
        self.suggestion_index = 0;
        self.input.clear();
        self.suggestions.clear();
    }

    fn execute_command(
        &mut self,
        command_line: &str,
        previous_command: Option<Child>,
        has_more_commands: bool,
    ) -> Result<Option<Child>, Box<dyn Error>> {
        if command_line.is_empty() {
            return Ok(None);
        }
        let parsed_command = self.parser.parse(&command_line);
        let command = parsed_command.command.as_str();

        match command {
            "cd" => {
                self.change_directory(&parsed_command.paths)?;
                Ok(None)
            }
            "exit" | "exit;" => {
                std::process::exit(0);
            }
            "about" => {
                print_about();
                Ok(None)
            }
            _ => {
                let stdin = self.get_stdin(previous_command);
                let stdout = self.get_stdout(has_more_commands);

                let resolved_command = self.resolve_path(command)?;

                let child = Command::new(resolved_command)
                    .args(parsed_command.args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .spawn()?;

                Ok(Some(child))
            }
        }
    }

    fn change_directory(&self, args: &[String]) -> Result<(), Box<dyn Error>> {
        let path = args.join("/");
        let root = Path::new(&path);
        env::set_current_dir(&root)?;
        Ok(())
    }

    fn resolve_path(&self, command: &str) -> Result<String, Box<dyn Error>> {
        if command.contains('/') {
            Ok(command.to_string())
        } else {
            let path = env::var("PATH").unwrap_or_default();
            let binary_locations = path.split(":").collect::<Vec<_>>();
            for location in binary_locations {
                let full_path: PathBuf = Path::new(location).join(command);
                if full_path.exists() {
                    return Ok(full_path.to_string_lossy().to_string());
                }
            }
            Err(format!("Command not found: {}", command).into())
        }
    }

    fn get_stdin(&self, previous_command: Option<Child>) -> Stdio {
        previous_command
            .and_then(|mut child| child.stdout.take())
            .map_or(Stdio::inherit(), Stdio::from)
    }

    fn get_stdout(&self, has_more_commands: bool) -> Stdio {
        if has_more_commands {
            Stdio::piped()
        } else {
            Stdio::inherit()
        }
    }
}
