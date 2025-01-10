use shell::Shell;

mod history;
mod shell;

fn main() {
    let mut shell = Shell::new();
    shell.init();
}
