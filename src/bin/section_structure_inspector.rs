use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::regex_reader::RegexReader;
use wikters::string_ops_reader::StringOpsReader;
use wikters::{PageSource, Opts};

#[derive(Debug, Parser)]
#[command(version, about = "Analyze heading structure patterns in Wiktionary dump")]
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

    /// Only analyze English sections
    #[clap(long)]
    english_only: bool,

    /// Show full patterns for each page (verbose)
    #[clap(long)]
    verbose: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum HeadingLevel {
    L2, // ==Language==
    L3, // ===Section===
    L4, // ====Subsection====
    L5, // =====Sub-subsection=====
}

impl HeadingLevel {
    fn from_equals(count: usize) -> Option<Self> {
        match count {
            2 => Some(HeadingLevel::L2),
            3 => Some(HeadingLevel::L3),
            4 => Some(HeadingLevel::L4),
            5 => Some(HeadingLevel::L5),
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        match self {
            HeadingLevel::L2 => "L2".to_string(),
            HeadingLevel::L3 => "L3".to_string(),
            HeadingLevel::L4 => "L4".to_string(),
            HeadingLevel::L5 => "L5".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct Heading {
    level: HeadingLevel,
    text: String,
}

fn extract_headings(text: &str) -> Vec<Heading> {
    let mut headings = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        
        // Count leading equals
        let leading_equals = trimmed.chars().take_while(|c| *c == '=').count();
        let trailing_equals = trimmed.chars().rev().take_while(|c| *c == '=').count();

        // Valid heading has matching leading and trailing equals, at least 2, and content between
        if leading_equals >= 2 && leading_equals == trailing_equals && leading_equals * 2 < trimmed.len() {
            if let Some(level) = HeadingLevel::from_equals(leading_equals) {
                let content = &trimmed[leading_equals..trimmed.len() - trailing_equals];
                let text = content.trim().to_string();
                if !text.is_empty() {
                    headings.push(Heading { level, text });
                }
            }
        }
    }

    headings
}

fn get_english_section(text: &str) -> Option<String> {
    let headings = extract_headings(text);
    
    // Find English section (L2 heading with "English")
    let english_start = headings.iter().position(|h| {
        h.level == HeadingLevel::L2 && h.text.to_lowercase().contains("english")
    })?;

    // Find the end (next L2 heading or end of text)
    let _english_end = headings[english_start + 1..]
        .iter()
        .position(|h| h.level == HeadingLevel::L2)
        .map(|p| p + english_start + 1)
        .unwrap_or(headings.len());

    // Extract the substring from English heading to next L2 (or end)
    let english_heading_line = text.lines().position(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("==") && !trimmed.starts_with("===") && 
        trimmed.contains("English")
    })?;

    let next_l2_line = text.lines().enumerate().skip(english_heading_line + 1).find(|(_, line)| {
        let trimmed = line.trim();
        trimmed.starts_with("==") && !trimmed.starts_with("===")
    }).map(|(idx, _)| idx);

    let lines: Vec<_> = text.lines().collect();
    let end_line = next_l2_line.unwrap_or(lines.len());
    Some(lines[english_heading_line..end_line].join("\n"))
}

fn analyze_english_structure(english_text: &str) -> String {
    let headings = extract_headings(english_text);
    
    // Skip the ==English== heading itself
    let inner_headings: Vec<_> = headings.into_iter().filter(|h| h.level != HeadingLevel::L2).collect();
    
    if inner_headings.is_empty() {
        return "EMPTY".to_string();
    }

    let mut pattern = Vec::new();
    for heading in &inner_headings {
        let is_etymology = heading.text.to_lowercase().contains("etymology");
        let is_pronunciation = heading.text.to_lowercase().contains("pronunciation");
        let is_pos = ["noun", "verb", "adjective", "adverb", "preposition", "conjunction", 
                      "interjection", "determiner", "pronoun", "article", "numeral"]
            .iter()
            .any(|pos| heading.text.to_lowercase().contains(pos));

        let label = if is_etymology {
            "Etymology"
        } else if is_pronunciation {
            "Pronunciation"
        } else if is_pos {
            "POS"
        } else {
            "Other"
        };

        pattern.push(format!("{}({})", label, heading.level.to_string()));
    }

    pattern.join(" -> ")
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

    // Choose reader
    let source: Box<dyn PageSource> = if args.stringops {
        Box::new(StringOpsReader::new(stdin.lock()))
    } else if args.handrolled {
        Box::new(RegexReader::new(stdin.lock()))
    } else {
        Box::new(QuickXmlReader::new(stdin.lock()))
    };

    let mut pattern_counts: HashMap<String, u32> = HashMap::new();
    let mut pages_with_english = 0;
    let mut pages_processed = 0;

    // Process pages
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

                // Skip non-article namespace
                if page.ns.unwrap_or(-1) != 0 {
                    continue;
                }

                // Get English section
                if let Some(english_text) = get_english_section(&page.rev_text) {
                    pages_with_english += 1;
                    
                    let pattern = analyze_english_structure(&english_text);
                    *pattern_counts.entry(pattern.clone()).or_insert(0) += 1;

                    if args.verbose && pages_with_english <= 20 {
                        println!("=== {} ===", page.title);
                        println!("{}", pattern);
                        println!();
                    }
                }
            }
            None => break,
        }
    }

    // Sort by count (descending)
    let mut sorted: Vec<_> = pattern_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    println!("English Section Structure Report");
    println!("({} pages scanned, {} with English sections)", pages_processed, pages_with_english);
    println!("==================================================");
    println!();

    for (pattern, count) in sorted.iter().take(30) {
        let pct = (**count as f64 / pages_with_english as f64) * 100.0;
        println!("{:3}% ({:5} pages) - {}", pct as u32, count, pattern);
    }

    println!();
    println!("==================================================");
    println!("Total unique patterns: {}", pattern_counts.len());

    Ok(())
}
