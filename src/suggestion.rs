pub fn get_command_suggestion(commands: &Vec<String>, input: &str) -> Vec<String> {
    let mut suggestions: Vec<String> = vec![];
    for command in commands {
        if command.starts_with(input) {
            suggestions.push(command.clone());
        }
    }
    suggestions
}
