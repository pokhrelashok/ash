use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::{env, error::Error};
fn main() {
    let mut input = String::new();
    let mut command_history: Vec<String> = vec![];
    loop {
        input.clear();
        let _ = collect_input(&mut input, &mut command_history);
        if let Err(e) = process_input(&input) {
            eprintln!("Error: {}", e);
        }
    }
}

fn collect_input(
    input: &mut String,
    command_history: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut index = command_history.len();
    print_prompt(input);
    loop {
        while let Ok(true) = event::poll(std::time::Duration::from_millis(500)) {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char(c) => handle_char_input(input, c)?,
                    KeyCode::Backspace => handle_backspace(input)?,
                    KeyCode::Enter => {
                        disable_raw_mode()?;
                        handle_enter(input, command_history);
                        return Ok(());
                    }
                    KeyCode::Up => {
                        if index > 0 {
                            if index == command_history.len() {
                                command_history.push(input.clone());
                            }
                            index -= 1;
                            handle_arrow(input, index, command_history)?
                        }
                    }
                    KeyCode::Down => {
                        if index < (command_history.len() - 1) {
                            index += 1;
                            handle_arrow(input, index, command_history)?;

                            if index == command_history.len() - 1 {
                                command_history.pop();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if input.trim() == "exit" {
            break;
        }
    }
    Ok(())
}

/// Prints the prompt and current input.
fn print_prompt(current_input: &str) {
    print!("\r\x1b[2K> {}", current_input); // Clear line and print
    stdout().flush().unwrap();
}

/// Handles character input by appending to the current input.
fn handle_char_input(
    current_input: &mut String,
    c: char,
) -> Result<(), Box<dyn std::error::Error>> {
    current_input.push(c);
    print_prompt(current_input);
    Ok(())
}

/// Handles backspace input by removing the last character.
fn handle_backspace(current_input: &mut String) -> Result<(), Box<dyn std::error::Error>> {
    if !current_input.is_empty() {
        current_input.pop();
    }
    print_prompt(current_input);
    Ok(())
}

/// Handles the Enter key by updating the command history.
fn handle_enter(current_input: &mut String, command_history: &mut Vec<String>) {
    println!(); // Move to the next line
    if !current_input.trim().is_empty() {
        command_history.push(current_input.clone());
    }
}

/// Handles the Up arrow key by fetching the last command from history.
fn handle_arrow(
    current_input: &mut String,
    current_index: usize,
    command_history: &Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(last_command) = command_history.iter().nth(current_index) {
        *current_input = last_command.clone();
    }
    print_prompt(current_input);
    Ok(())
}

/// Processes the entire input string, splitting commands and managing pipes.
fn process_input(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut commands = input.split(" | ").peekable();
    let mut previous_command: Option<Child> = None;

    while let Some(command) = commands.next() {
        previous_command =
            execute_command(command.trim(), previous_command, commands.peek().is_some())?;
    }

    if let Some(mut final_command) = previous_command {
        final_command.wait()?;
    }

    Ok(())
}

/// Executes a single command, handling input/output redirection and piping.
fn execute_command(
    command_line: &str,
    previous_command: Option<Child>,
    has_more_commands: bool,
) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    let mut parts = command_line.split_whitespace();
    let command = parts.next().ok_or("Empty command")?;
    let args: Vec<&str> = parts.collect();

    match command {
        "cd" => {
            change_directory(&args)?;
            Ok(None)
        }
        "exit" => {
            std::process::exit(0);
        }
        command => {
            let stdin = get_stdin(previous_command);
            let stdout = get_stdout(has_more_commands);

            let resolved_command = resolve_command(command)?;

            let child = Command::new(resolved_command)
                .args(args)
                .stdin(stdin)
                .stdout(stdout)
                .spawn()?;

            Ok(Some(child))
        }
    }
}

/// Changes the current working directory.
fn change_directory(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let new_dir = args.get(0).map_or("/", |&x| x);
    let root = Path::new(new_dir);
    env::set_current_dir(&root)?;
    Ok(())
}

/// Resolves the full path of a command if necessary.
fn resolve_command(command: &str) -> Result<String, Box<dyn std::error::Error>> {
    if command.contains('/') {
        Ok(command.to_string())
    } else {
        let binary_locations = vec!["/bin", "/usr/bin"]; // Example binary paths
        for location in binary_locations {
            let full_path: PathBuf = Path::new(location).join(command);
            if full_path.exists() {
                return Ok(full_path.to_string_lossy().to_string());
            }
        }
        Err(format!("Command not found: {}", command).into())
    }
}

/// Gets the `Stdio` for the `stdin` of a command, based on the previous command's output.
fn get_stdin(previous_command: Option<Child>) -> Stdio {
    previous_command
        .and_then(|mut child| child.stdout.take())
        .map_or(Stdio::inherit(), Stdio::from)
}

/// Gets the `Stdio` for the `stdout` of a command, based on whether more commands exist.
fn get_stdout(has_more_commands: bool) -> Stdio {
    if has_more_commands {
        Stdio::piped()
    } else {
        Stdio::inherit()
    }
}
