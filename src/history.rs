use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub struct History {
    pub fd: File,
    pub commands: Vec<String>,
}

impl History {
    pub fn new(path: String) -> History {
        let hist_path = Path::new(path.as_str());
        if !hist_path.exists() {
            let file = File::create(path.clone()).unwrap();
            History {
                fd: file,
                commands: vec![],
            }
        } else {
            let mut file = File::options()
                .read(true)
                .write(true)
                .open(path.clone())
                .unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            let commands = contents.split("\n").map(|f| f.to_string()).collect();
            History { fd: file, commands }
        }
    }
}

impl Drop for History {
    fn drop(&mut self) {
        let _ = self
            .fd
            .write_all(self.commands.join("\n").as_bytes())
            .expect("Cannot write history");
    }
}
