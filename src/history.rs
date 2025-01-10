use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::PathBuf,
};

pub struct History {
    path: PathBuf,
    commands: Vec<String>,
}

impl History {
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();

        if !path.exists() {
            File::create(&path)?;
        }

        let mut file = File::open(&path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let commands = contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(String::from)
            .collect();

        Ok(Self { path, commands })
    }

    pub fn add_command(&mut self, command: &str) {
        if self.commands.last().map_or("", |f| f) != command {
            self.commands.push(command.to_string());
        }
    }

    pub fn get_command(&self, index: usize) -> Option<&String> {
        self.commands.get(index)
    }

    pub fn count(&self) -> usize {
        self.commands.len()
    }
}

impl Drop for History {
    fn drop(&mut self) {
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "{}", self.commands.join("\n"));
        }
    }
}
