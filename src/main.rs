use std::{
    env,
    io::{stdin, stdout, Write},
    path::Path,
    process::Command,
};

fn main() {
    let binary_locations = ["/bin", "/usr/bin", "/usr/local/bin"];
    let mut input = String::new();
    loop {
        print!("$ ");
        stdout().flush().unwrap();
        input.clear();
        stdin().read_line(&mut input).unwrap();
        let mut parts = input.trim().split_whitespace();
        let mut command = parts.next().unwrap();
        let args = parts;
        match command {
            "cd" => {
                let new_dir = args.peekable().peek().map_or("/", |x| *x);
                let root = Path::new(new_dir);
                if let Err(e) = env::set_current_dir(&root) {
                    eprintln!("{}", e);
                }
            }
            "exit" => return,
            command => {
                let mut command = command.to_string();
                let mut found = true;
                if !command.contains('/') {
                    found = false;
                    for ele in binary_locations {
                        let full_path = Path::new(ele).join(&command);
                        if full_path.exists() {
                            command = full_path.to_string_lossy().to_string();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        eprintln!("Command not found!");
                    }
                }
                if found {
                    let child = Command::new(&command).args(args).spawn();
                    match child {
                        Ok(mut child) => {
                            child.wait();
                        }
                        Err(e) => {
                            eprintln!("{}", e)
                        }
                    };
                }
            }
        }
    }
}
