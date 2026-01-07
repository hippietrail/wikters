use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::regex_reader::RegexReader;
use wikters::string_ops_reader::StringOpsReader;
use wikters::{PageSource, Opts};

#[derive(Debug, Parser)]
#[command(version, about = "Distinguish top-level vs nested Pronunciation patterns")]
struct Args {
    /// Limit the number of pages to scan
    #[clap(short, long)]
    limit: Option<u64>,

    /// Use regex-based hand-rolled parser
    #[clap(short = 'r', long)]
    handrolled: bool,

    /// Use string-ops hand-rolled parser
    #[clap(short = 's', long)]
    stringops: bool,

    /// Show examples
    #[clap(long)]
    examples: bool,
}

fn count_leading_equals(s: &str) -> usize {
    s.chars().take_while(|c| *c == '=').count()
}

fn is_valid_heading(line: &str) -> bool {
    let trimmed = line.trim();
    let leading = trimmed.chars().take_while(|c| *c == '=').count();
    let trailing = trimmed.chars().rev().take_while(|c| *c == '=').count();
    leading >= 2 && leading == trailing && leading * 2 < trimmed.len()
}

fn get_heading_text(line: &str) -> String {
    let trimmed = line.trim();
    let leading = trimmed.chars().take_while(|c| *c == '=').count();
    let trailing = trimmed.chars().rev().take_while(|c| *c == '=').count();
    trimmed[leading..trimmed.len() - trailing].trim().to_string()
}

fn get_english_section(text: &str) -> Option<(usize, usize)> {
    let lines: Vec<_> = text.lines().collect();
    
    let english_start = lines.iter().position(|line| {
        let trimmed = line.trim();
        is_valid_heading(trimmed) && 
        count_leading_equals(trimmed) == 2 &&
        trimmed.contains("English")
    })?;

    let english_end = lines[english_start + 1..]
        .iter()
        .position(|line| {
            let trimmed = line.trim();
            is_valid_heading(trimmed) && count_leading_equals(trimmed) == 2
        })
        .map(|p| p + english_start + 1)
        .unwrap_or(lines.len());

    Some((english_start, english_end))
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum PronunciationPattern {
    TopLevelL3,           // ===Pronunciation=== at L3
    NestedL4UnderEtymology, // ====Pronunciation==== under ===Etymology===
    Both,                 // Both patterns present
    Neither,
}

fn is_etymology_section(text: &str) -> bool {
    let lower = text.to_lowercase();
    // Matches "Etymology", "Etymology 1", "Etymology 2", etc
    lower.starts_with("etymology") || lower.contains(" etymology")
}

fn is_pronunciation_section(text: &str) -> bool {
    let lower = text.to_lowercase();
    // Matches "Pronunciation", "Pronunciation 1", "Pronunciation 2", etc
    lower.starts_with("pronunciation") || lower.contains(" pronunciation")
}

fn analyze_pronunciation_pattern(text: &str) -> PronunciationPattern {
    let lines: Vec<_> = text.lines().collect();
    let (start, end) = match get_english_section(text) {
        Some(range) => range,
        None => return PronunciationPattern::Neither,
    };

    let mut has_l3_pronunciation = false;
    let mut has_l4_pronunciation_under_etymology = false;
    let mut last_l3_type = String::new();

    for i in start + 1..end {
        let line = lines[i];
        let trimmed = line.trim();
        
        if !is_valid_heading(trimmed) {
            continue;
        }

        let level = count_leading_equals(trimmed);
        let heading_text = get_heading_text(line);

        if level == 3 {
            last_l3_type = heading_text.clone();
            if is_pronunciation_section(&heading_text) {
                has_l3_pronunciation = true;
            }
        } else if level == 4 && is_pronunciation_section(&heading_text) {
            if is_etymology_section(&last_l3_type) {
                has_l4_pronunciation_under_etymology = true;
            }
        }
    }

    match (has_l3_pronunciation, has_l4_pronunciation_under_etymology) {
        (true, true) => PronunciationPattern::Both,
        (true, false) => PronunciationPattern::TopLevelL3,
        (false, true) => PronunciationPattern::NestedL4UnderEtymology,
        (false, false) => PronunciationPattern::Neither,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let opts = Opts {
        limit: args.limit,
        xml: false,
        no_updates: false,
        sample_rate: None,
        handrolled: args.handrolled,
    };

    let stdin = io::stdin();

    let source: Box<dyn PageSource> = if args.stringops {
        Box::new(StringOpsReader::new(stdin.lock()))
    } else if args.handrolled {
        Box::new(RegexReader::new(stdin.lock()))
    } else {
        Box::new(QuickXmlReader::new(stdin.lock()))
    };

    let mut pattern_counts: HashMap<PronunciationPattern, (u32, Vec<String>)> = HashMap::new();
    let mut pages_processed = 0;

    let mut source = source;
    loop {
        if let Some(limit) = opts.limit {
            if pages_processed >= limit {
                break;
            }
        }

        match source.next_page()? {
            Some(page) => {
                pages_processed += 1;

                if page.ns.unwrap_or(-1) != 0 {
                    continue;
                }

                let pattern = analyze_pronunciation_pattern(&page.rev_text);
                let entry = pattern_counts.entry(pattern).or_insert((0, Vec::new()));
                entry.0 += 1;
                if args.examples && entry.1.len() < 3 {
                    entry.1.push(page.title.clone());
                }
            }
            None => break,
        }
    }

    let mut sorted: Vec<_> = pattern_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    println!("Pronunciation Nesting Pattern Analysis");
    println!("({} pages scanned)", pages_processed);
    println!("==================================================");
    println!();

    for (pattern, (count, examples)) in sorted.iter() {
        let pct = (*count as f64 / pages_processed as f64) * 100.0;
        println!("{:3}% ({:6} pages) - {:?}", pct as u32, count, pattern);
        if args.examples && !examples.is_empty() {
            println!("               Examples: {}", examples.join(", "));
        }
        println!();
    }

    println!("==================================================");
    println!("Key insight:");
    println!("- TopLevelL3: ===Pronunciation=== shared across homographs");
    println!("- NestedL4: ====Pronunciation==== per etymology branch");
    println!("- Both: Page has both patterns (complex structure)");

    Ok(())
}
