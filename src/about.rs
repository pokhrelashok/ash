use std::{env, fs};

pub fn print_about() {
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
