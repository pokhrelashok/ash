use crossterm::{
    cursor::{MoveLeft, MoveRight},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::{env, error::Error};
use std::{
    fs::{self},
    io::stdout,
};

use crate::{autocomplete::AutoComplete, history::History};

pub struct Shell {
    input: String,
    temp_input: String,
    history: History,
    stdout: Stdout,
    autocompleter: AutoComplete,
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
        let mut index: i32 = -1;
        self.print_prompt();

        loop {
            if let Ok(true) = event::poll(std::time::Duration::from_millis(500)) {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                        && key_event.code == KeyCode::Char('c')
                    {
                        self.input.clear();
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
                            if index < (self.history.count() - 1) as i32 {
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
                            execute!(self.stdout, MoveLeft(1)).unwrap();
                        }
                        KeyCode::Right => {
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
        match self.autocompleter.autocomplete(self.input.as_str()) {
            Ok(new_command) => {
                self.input = new_command;
                self.print_prompt();
            }
            Err(_) => todo!(),
        }
        enable_raw_mode()?;
        Ok(())
    }

    fn print_prompt(&self) {
        let cwd = env::current_dir()
            .unwrap_or_default()
            .into_os_string()
            .into_string()
            .unwrap_or("".to_string());
        let wdir = cwd.split("/").last().unwrap_or_default();
        print!("\r\x1b[2K{}{}  {}", " ", wdir, self.input);
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
        let parsed_command = self.autocompleter.parser.parse(&command_line);
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
                self.about();
                Ok(None)
            }
            _ => {
                let stdin = self.get_stdin(previous_command);
                let stdout = self.get_stdout(has_more_commands);

                let resolved_command = self.resolve_command(command)?;

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
