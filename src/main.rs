use shell::Shell;

mod shell;

fn main() {
    let mut shell = Shell::new();
    shell.init();
}
