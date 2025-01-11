use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use regex::Regex;
use std::fs::{self};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::{env, error::Error};

use crate::history::History;

pub struct Shell {
    input: String,
    temp_input: String,
    history: History,
    was_last_up: bool,
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
            input: "".to_string(),
            temp_input: "".to_string(),
            history,
            was_last_up: true,
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
        }
    }

    fn collect_input(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut index = 0;
        self.print_prompt();

        loop {
            if let Ok(true) = event::poll(std::time::Duration::from_millis(500)) {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                        && key_event.code == KeyCode::Char('c')
                    {
                        self.input.clear();
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
                            if index < self.history.count() {
                                if !self.was_last_up && index != 0 {
                                    index -= 1
                                }
                                self.was_last_up = true;
                                if index == 0 {
                                    self.temp_input = self.input.clone();
                                }
                                if index == self.history.count() - 2 {
                                    self.history.fetch_more();
                                }
                                self.handle_arrow(index)?;
                                index += 1
                            }
                        }
                        KeyCode::Down => {
                            if index > 0 {
                                if self.was_last_up {
                                    index -= 1;
                                }
                                self.was_last_up = false;
                                index -= 1;
                                self.handle_arrow(index)?;
                            } else if index == 0 {
                                self.input = self.temp_input.clone();
                                self.print_prompt();
                            }
                        }
                        KeyCode::Tab => {
                            self.autocomplete()?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn autocomplete(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        let part_command = self
            .input
            .split_whitespace()
            .nth(0)
            .unwrap_or("")
            .to_string();

        let mut part_filename = self
            .input
            .split_whitespace()
            .last()
            .unwrap_or("")
            .to_string();

        // Replace `~` with the user's home directory
        if part_filename.starts_with('~') {
            part_filename = part_filename.replace(
                "~",
                &format!(
                    "/home/{}",
                    env::var("USER").unwrap_or_else(|_| "Unknown".to_string())
                ),
            );
        } else if !part_filename.starts_with(".") {
            part_filename = String::from("./") + &part_filename;
        }
        let regex_filename = Regex::new(r"[0-9a-zA-Z]").unwrap();
        let part_filepath = Regex::new(r"[~./]").unwrap();

        let searched_file = part_filepath.replace_all(&part_filename, "").to_string();
        part_filename = regex_filename.replace_all(&part_filename, "").to_string();

        let mut entries = fs::read_dir(&part_filename)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;
        entries.sort();

        if part_command == "cd" {
            entries = entries.into_iter().filter(|f| f.is_dir()).collect();
        }

        let terminal_width = terminal::size()?.0 as usize;

        let mut matching_file_names: Vec<String> = vec![];

        for (_i, entry) in entries.iter().enumerate() {
            let file_name = entry.file_name().unwrap().to_string_lossy().to_string();
            if searched_file.len() == 0 || file_name.starts_with(&searched_file) {
                matching_file_names.push(file_name.clone());
            }
        }

        if matching_file_names.len() > 1 {
            let max_width = entries
                .iter()
                .map(|entry| entry.file_name().unwrap().to_string_lossy().len())
                .max()
                .unwrap_or(0);
            let columns = (terminal_width / (max_width + 2)).max(1); // Add 4 for padding
            println!("");

            for (i, value) in matching_file_names.iter().enumerate() {
                print!("{:<width$}", value, width = max_width + 4);
                // Break line after the last column
                if (i + 1) % columns == 0 {
                    println!();
                }
            }
            // Ensure we end with a new line
            if entries.len() % columns != 0 {
                println!();
            }
        } else if matching_file_names.len() == 1 {
            let matched = matching_file_names
                .first()
                .unwrap_or(&"".to_string())
                .to_string();
            self.input = self.input.replace(&searched_file, &matched) + "/";
        }
        self.print_prompt();
        enable_raw_mode()?;
        Ok(())
    }

    fn print_prompt(&self) {
        let cwd = env::current_dir()
            .unwrap_or_default()
            .into_os_string()
            .into_string()
            .unwrap_or("".to_string());
        print!("\r\x1b[2K{}> {}", cwd, self.input);
        io::stdout().flush().unwrap();
    }

    fn handle_char_input(&mut self, c: char) -> Result<(), Box<dyn Error>> {
        self.input.push(c);
        self.print_prompt();
        Ok(())
    }

    fn handle_backspace(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.input.is_empty() {
            self.input.pop();
        }
        self.print_prompt();
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
        &mut self,
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
            "about" => {
                self.about();
                Ok(None)
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
    fn about(&self) {
        let ascii_art = r#"⠀⠀⠀⠀⠀⣀⣠⣤⣤⣤⣤⣄⣀⠀⠀⠀⠀⠀
⠀⠀⢀⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣦⡀⠀⠀
⠀⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠟⢿⣿⣷⡀⠀
⣸⣿⣿⣿⣿⣿⣿⣿⣿⣿⠟⠁⠀⣴⢿⣿⣧⠀
⣿⣿⣿⣿⣿⡿⠛⣩⠍⠀⠀⠀⠐⠉⢠⣿⣿⡇
⣿⡿⠛⠋⠉⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣿⣿
⢹⣿⣤⠄⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣿⣿⡏
⠀⠻⡏⠀⠀⠀⠀⠀⠀⠀⠀⠀⢿⣿⣿⣿⠟⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢻⠟⠁⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀"#;

        // Fetch system information
        let username = env::var("USER").unwrap_or_else(|_| "Unknown".to_string());
        let hostname = env::var("HOSTNAME").unwrap_or_else(|_| {
            fs::read_to_string("/etc/hostname")
                .unwrap_or_else(|_| "Unknown".to_string())
                .trim()
                .to_string()
        });
        let os = fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|line| line.starts_with("PRETTY_NAME="))
                    .map(|line| line.replace("PRETTY_NAME=", "").replace('"', ""))
            })
            .unwrap_or_else(|| "Unknown".to_string());
        let kernel = fs::read_to_string("/proc/version")
            .map(|v| v.split_whitespace().nth(2).unwrap_or("Unknown").to_string())
            .unwrap_or_else(|_| "Unknown".to_string());
        let uptime = fs::read_to_string("/proc/uptime")
            .map(|up| {
                up.split_whitespace()
                    .next()
                    .and_then(|secs| secs.parse::<f64>().ok())
                    .map(|s| format!("{:.2} hours", s / 3600.0))
                    .unwrap_or("Unknown".to_string())
            })
            .unwrap_or_else(|_| "Unknown".to_string());

        // RAM Information (Total)
        let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let total_ram = meminfo
            .lines()
            .find(|line| line.starts_with("MemTotal:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|kb| kb.parse::<u64>().ok())
            .map(|kb| format!("{:.2} GB", kb as f64 / (1024.0 * 1024.0)))
            .unwrap_or_else(|| "Unknown".to_string());

        // CPU Model
        let cpu_model = fs::read_to_string("/proc/cpuinfo")
            .unwrap_or_default()
            .lines()
            .find(|line| line.starts_with("model name"))
            .map(|line| {
                line.split(':')
                    .nth(1)
                    .unwrap_or("Unknown")
                    .trim()
                    .to_string()
            })
            .unwrap_or_else(|| "Unknown".to_string());

        let sh = env::var("0").unwrap_or_else(|_| {
            fs::read_to_string("/etc/passwd")
                .unwrap_or_default()
                .lines()
                .find(|line| line.contains(&username))
                .map(|line| {
                    line.split(":")
                        .last()
                        .unwrap_or("Unknown")
                        .split("/")
                        .last()
                        .unwrap_or("Unknown")
                        .to_string()
                })
                .unwrap_or_else(|| "Unknown".to_string())
        });

        // Collect system info
        let system_info = vec![
            format!("User:    {}", username),
            format!("Host:    {}", hostname),
            format!("OS:      {}", os),
            format!("Kernel:  {}", kernel),
            format!("Uptime:  {}", uptime),
            format!("RAM:     {}", total_ram),
            format!("CPU:     {}", cpu_model),
            format!("Shell:   {}", sh),
        ];

        // Print ASCII art and information side-by-side
        let art_lines: Vec<&str> = ascii_art.lines().collect();
        let info_lines = system_info;

        let max_art_width = art_lines.iter().map(|line| line.len()).max().unwrap_or(0) + 5;

        for (i, art_line) in art_lines.iter().enumerate() {
            print!("{}", art_line);
            if i < info_lines.len() {
                print!(
                    "{:width$}{}",
                    "",
                    info_lines[i],
                    width = max_art_width - art_line.len()
                );
            }
            println!();
        }

        // Print remaining info lines if any
        if art_lines.len() < info_lines.len() {
            for line in info_lines.iter().skip(art_lines.len()) {
                println!("{:width$}{}", "", line, width = max_art_width);
            }
        }
    }
}
