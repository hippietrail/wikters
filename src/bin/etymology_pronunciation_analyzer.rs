use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::regex_reader::RegexReader;
use wikters::string_ops_reader::StringOpsReader;
use wikters::{PageSource, Opts};

#[derive(Debug, Parser)]
#[command(version, about = "Analyze Etymology/Pronunciation/POS nesting patterns in English sections")]
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

    /// Show examples of each pattern
    #[clap(long)]
    examples: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum SectionType {
    Etymology,
    Pronunciation,
    POS(String), // noun, verb, etc
    Other(String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Section {
    section_type: SectionType,
    level: u32,
    children: Vec<Section>,
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

fn classify_section(text: &str) -> SectionType {
    let lower = text.to_lowercase();
    
    if lower.contains("etymology") {
        SectionType::Etymology
    } else if lower.contains("pronunciation") {
        SectionType::Pronunciation
    } else {
        let pos_types = [
            "noun", "verb", "adjective", "adverb", "preposition", "conjunction",
            "interjection", "determiner", "pronoun", "article", "numeral",
        ];
        for pos in &pos_types {
            if lower.contains(pos) {
                return SectionType::POS(pos.to_string());
            }
        }
        SectionType::Other(text.to_string())
    }
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

fn analyze_english_structure(text: &str) -> Option<String> {
    let lines: Vec<_> = text.lines().collect();
    let (start, end) = get_english_section(text)?;

    let mut structure = Vec::new();
    let mut last_level = 2;

    for i in start + 1..end {
        let line = lines[i];
        let trimmed = line.trim();
        
        if !is_valid_heading(trimmed) {
            continue;
        }

        let level = count_leading_equals(trimmed);
        let heading_text = get_heading_text(line);
        let section_type = classify_section(&heading_text);

        let indent = if level > last_level { "  " } else { "" };
        
        let type_str = match section_type {
            SectionType::Etymology => "Etymology".to_string(),
            SectionType::Pronunciation => "Pronunciation".to_string(),
            SectionType::POS(pos) => format!("{}", pos),
            SectionType::Other(s) => format!("Other({})", s.split_whitespace().next().unwrap_or("?")),
        };

        structure.push(format!("{}L{}:{}", indent, level, type_str));
        last_level = level;
    }

    if structure.is_empty() {
        None
    } else {
        Some(structure.join(" | "))
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

    let mut structure_counts: HashMap<String, (u32, Vec<String>)> = HashMap::new();
    let mut pages_with_english = 0;
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

                if let Some(structure) = analyze_english_structure(&page.rev_text) {
                    pages_with_english += 1;
                    
                    let entry = structure_counts.entry(structure).or_insert((0, Vec::new()));
                    entry.0 += 1;
                    if args.examples && entry.1.len() < 3 {
                        entry.1.push(page.title);
                    }
                }
            }
            None => break,
        }
    }

    let mut sorted: Vec<_> = structure_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    println!("English Section Structure Analysis");
    println!("({} pages scanned, {} with English sections)", pages_processed, pages_with_english);
    println!("==================================================");
    println!();

    for (structure, (count, examples)) in sorted.iter().take(40) {
        let pct = (*count as f64 / pages_with_english as f64) * 100.0;
        println!("{:3}% ({:5} pages)", pct as u32, count);
        println!("  {}", structure);
        if args.examples && !examples.is_empty() {
            println!("  Examples: {}", examples.join(", "));
        }
        println!();
    }

    println!("==================================================");
    println!("Total unique structures: {}", structure_counts.len());

    Ok(())
}
