use shell::Shell;
mod about;
mod autocomplete;
mod history;
mod parser;
mod shell;
mod suggestion;
extern crate toml;
fn main() {
    let shell = Shell::new();
    match shell {
        Ok(mut app) => app.init(),
        Err(e) => println!("Cannot init {:?}", e),
    }
}
