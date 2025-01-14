use shell::Shell;
mod history;
mod parser;
mod shell;
extern crate toml;
fn main() {
    let shell = Shell::new();
    match shell {
        Ok(mut app) => app.init(),
        Err(e) => println!("Cannot init {:?}", e),
    }
}
