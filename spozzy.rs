use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn extract_unordered_key(line: &str) -> &str {
    match line.find('\t') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn extract_ordered_key(mut line: &str) -> &str {
    // Remove comment if present (whitespace + # + rest)
    if let Some(idx) = line.find('#') {
        // Only strip if the # is preceded by whitespace
        if idx > 0 && line[..idx].chars().rev().next().unwrap().is_whitespace() {
            line = &line[..idx].trim_end();
        }
    }
    // If there's a '/', take only the part before it
    match line.find('/') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <ordered_file> <unordered_file>", args[0]);
        std::process::exit(1);
    }

    // Read ordered file, filter, and collect keys
    let ordered_file = File::open(&args[1]).expect("Failed to open ordered file");
    let mut ordered_entries: Vec<(String, String)> = BufReader::new(ordered_file)
    .lines()
    .map(|l| l.expect("Failed to read line"))
    .filter(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    })
    .map(|line| {
        let key = extract_ordered_key(&line).to_string();
        (key, line)
    })
    .collect();
    ordered_entries.sort_by(|a, b| a.0.cmp(&b.0));

    let unordered_file = File::open(&args[2]).expect("Failed to open unordered file");
    for line in BufReader::new(unordered_file).lines() {
        let unordered_line = line.expect("Failed to read line");
        let key = extract_unordered_key(&unordered_line);
        if let Ok(idx) = ordered_entries.binary_search_by(|entry| entry.0.as_str().cmp(key)) {
            let ordered_line = &ordered_entries[idx].1;
            // println!("W: {}", unordered_line);
            // println!("H: {}", ordered_line);
            println!("{}\t\t{}", unordered_line, ordered_line);
        }
    }
}
