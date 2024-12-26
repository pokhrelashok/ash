use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

pub struct Shell {
    command_history: Vec<String>,
}

impl Shell {
    pub fn new() -> Shell {
        Shell {
            command_history: vec![],
        }
    }

    pub fn init(&mut self) {
        let mut input = String::new();
        loop {
            input.clear();
            if let Err(e) = self.collect_input(&mut input) {
                eprintln!("Error collecting input: {}", e);
                continue;
            }

            if input.trim() == "exit" {
                break;
            }

            if let Err(e) = self.process_input(&input) {
                eprintln!("Error processing input: {}", e);
            }
        }
    }

    fn collect_input(&mut self, input: &mut String) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut index = self.command_history.len();
        self.print_prompt(input);

        loop {
            if let Ok(true) = event::poll(std::time::Duration::from_millis(500)) {
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Char(c) => self.handle_char_input(input, c)?,
                        KeyCode::Backspace => self.handle_backspace(input)?,
                        KeyCode::Enter => {
                            disable_raw_mode()?;
                            self.handle_enter(input);
                            return Ok(());
                        }
                        KeyCode::Up => {
                            if index > 0 {
                                index -= 1;
                                self.handle_arrow(input, index)?;
                            }
                        }
                        KeyCode::Down => {
                            if index < self.command_history.len() {
                                index += 1;
                                self.handle_arrow(input, index)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn print_prompt(&self, current_input: &str) {
        print!("\r\x1b[2K> {}", current_input);
        io::stdout().flush().unwrap();
    }

    fn handle_char_input(&self, current_input: &mut String, c: char) -> Result<(), Box<dyn Error>> {
        current_input.push(c);
        self.print_prompt(current_input);
        Ok(())
    }

    fn handle_backspace(&self, current_input: &mut String) -> Result<(), Box<dyn Error>> {
        if !current_input.is_empty() {
            current_input.pop();
        }
        self.print_prompt(current_input);
        Ok(())
    }

    fn handle_enter(&mut self, current_input: &mut String) {
        println!();
        if !current_input.trim().is_empty() {
            if self.command_history.len() == 0
                || self
                    .command_history
                    .last()
                    .is_some_and(|x| x != current_input)
            {
                self.command_history.push(current_input.clone());
            }
        }
    }

    fn handle_arrow(&self, current_input: &mut String, index: usize) -> Result<(), Box<dyn Error>> {
        if index < self.command_history.len() {
            *current_input = self.command_history[index].clone();
            self.print_prompt(current_input);
        }
        Ok(())
    }

    fn process_input(&self, input: &str) -> Result<(), Box<dyn Error>> {
        let mut commands = input.split(" | ").peekable();
        let mut previous_command: Option<Child> = None;

        while let Some(command) = commands.next() {
            previous_command =
                self.execute_command(command.trim(), previous_command, commands.peek().is_some())?;
        }

        if let Some(mut final_command) = previous_command {
            final_command.wait()?;
        }

        Ok(())
    }

    fn execute_command(
        &self,
        command_line: &str,
        previous_command: Option<Child>,
        has_more_commands: bool,
    ) -> Result<Option<Child>, Box<dyn Error>> {
        if command_line.is_empty() {
            return Ok(None);
        }

        let mut parts = command_line.split_whitespace();
        let command = parts.next().ok_or("Empty command")?;
        let args: Vec<&str> = parts.collect();

        match command {
            "cd" => {
                self.change_directory(&args)?;
                Ok(None)
            }
            "exit" | "exit;" => {
                std::process::exit(0);
            }
            _ => {
                let stdin = self.get_stdin(previous_command);
                let stdout = self.get_stdout(has_more_commands);

                let resolved_command = self.resolve_command(command)?;

                let child = Command::new(resolved_command)
                    .args(args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .spawn()?;

                Ok(Some(child))
            }
        }
    }

    fn change_directory(&self, args: &[&str]) -> Result<(), Box<dyn Error>> {
        let new_dir = args.get(0).map_or("/", |&x| x);
        let root = Path::new(new_dir);
        env::set_current_dir(&root)?;
        Ok(())
    }

    fn resolve_command(&self, command: &str) -> Result<String, Box<dyn Error>> {
        if command.contains('/') {
            Ok(command.to_string())
        } else {
            let binary_locations = vec!["/bin", "/usr/bin"];
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
