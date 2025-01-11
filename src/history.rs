use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Seek, SeekFrom, Write},
    path::PathBuf,
};

pub struct History {
    path: PathBuf,
    reader: LineReader,
    commands: Vec<String>,
    new_commands_count: u32,
}

impl History {
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();

        if !path.exists() {
            File::create(&path)?;
        }

        let mut reader = LineReader::new(&path)?;
        let commands = reader.read_lines(10)?;

        Ok(Self {
            path,
            commands,
            reader,
            new_commands_count: 0,
        })
    }

    pub fn add_command(&mut self, command: &str) {
        if self.commands.first().map_or("", |f| f) != command {
            self.commands.insert(0, command.to_string());
            self.new_commands_count += 1;
        }
    }

    pub fn get_command(&self, index: usize) -> Option<&String> {
        self.commands.get(index)
    }

    pub fn fetch_more(&mut self) {
        match self.reader.read_lines(10) {
            Ok(mut cmds) => {
                if cmds.len() > 0 {
                    self.commands.append(&mut cmds);
                }
            }
            Err(_) => (),
        }
    }

    pub fn count(&self) -> usize {
        self.commands.len()
    }
}

impl Drop for History {
    fn drop(&mut self) {
        if let Ok(mut file) = OpenOptions::new().write(true).append(true).open(&self.path) {
            let _ = writeln!(
                file,
                "{}",
                self.commands
                    .iter()
                    .filter(|f| !f.is_empty())
                    .enumerate()
                    .filter(|(i, _)| *i < self.new_commands_count as usize)
                    .map(|(_, a)| a.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
}

pub struct LineReader {
    reader: BufReader<File>,
    position: u64,
}

impl LineReader {
    pub fn new(path: &PathBuf) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(Self {
            reader,
            position: 0,
        })
    }

    pub fn read_lines(&mut self, count: usize) -> io::Result<Vec<String>> {
        let mut lines = Vec::new();
        let _ = self.reader.seek(SeekFrom::Start(self.position));
        for _ in 0..count {
            let mut line = String::new();
            let bytes_read = self.reader.read_line(&mut line)?;
            if bytes_read == 0 {
                break;
            } else {
                self.position += bytes_read as u64;
            }
            lines.push(line.trim_end().to_string());
        }
        Ok(lines)
    }
}
