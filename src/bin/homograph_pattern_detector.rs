use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::regex_reader::RegexReader;
use wikters::string_ops_reader::StringOpsReader;
use wikters::{PageSource, Opts};

#[derive(Debug, Parser)]
#[command(version, about = "Detect homograph patterns: Etymology (L3) with nested POS (L4) vs flat POS (L3)")]
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

fn is_pos_heading(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "noun", "verb", "adjective", "adverb", "preposition", "conjunction",
        "interjection", "determiner", "pronoun", "article", "numeral",
    ]
    .iter()
    .any(|pos| lower.contains(pos))
}

fn is_etymology_heading(text: &str) -> bool {
    text.to_lowercase().contains("etymology")
}

fn is_pronunciation_heading(text: &str) -> bool {
    text.to_lowercase().contains("pronunciation")
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
enum HomographPattern {
    MultipleEtymologiesWithNestedPos, // L3:Etymology -> L4:POS (multiple etymologies)
    FlatPos,                           // L3:POS (no Etymology)
    SingleEtymologyWithFlatPos,        // L3:Etymology -> L3:POS
    PronunciationDividesHomographs,   // L3:Pronunciation (dividing) -> L4:Etymology -> L3/L4:POS
    Other(String),
}

fn classify_english_structure(text: &str) -> HomographPattern {
    let lines: Vec<_> = text.lines().collect();
    let (start, end) = match get_english_section(text) {
        Some(range) => range,
        None => return HomographPattern::Other("no_english_section".to_string()),
    };

    let mut headings: Vec<(usize, usize, String)> = Vec::new(); // (level, line_index, text)
    
    for i in start + 1..end {
        let line = lines[i];
        let trimmed = line.trim();
        
        if !is_valid_heading(trimmed) {
            continue;
        }

        let level = count_leading_equals(trimmed);
        let heading_text = get_heading_text(line);
        headings.push((level, i, heading_text));
    }

    if headings.is_empty() {
        return HomographPattern::Other("no_headings".to_string());
    }

    // Count L3 etymologies and their child L4:POS
    let mut l3_etymology_count = 0;
    let mut has_l4_pos_under_etymology = false;
    let mut has_l3_pos = false;
    let mut l3_pronunciation_dividers = 0;

    for i in 0..headings.len() {
        let (level, _idx, text) = &headings[i];
        
        if *level == 3 && is_etymology_heading(text) {
            l3_etymology_count += 1;
            
            // Check if next L4 is POS
            if i + 1 < headings.len() && headings[i + 1].0 == 4 && is_pos_heading(&headings[i + 1].2) {
                has_l4_pos_under_etymology = true;
            }
        }
        
        if *level == 3 && is_pos_heading(text) {
            has_l3_pos = true;
        }
        
        if *level == 3 && is_pronunciation_heading(text) {
            l3_pronunciation_dividers += 1;
        }
    }

    // Decision tree
    if l3_pronunciation_dividers > 0 {
        return HomographPattern::PronunciationDividesHomographs;
    }

    if l3_etymology_count >= 2 && has_l4_pos_under_etymology {
        return HomographPattern::MultipleEtymologiesWithNestedPos;
    }

    if l3_etymology_count == 0 && has_l3_pos {
        return HomographPattern::FlatPos;
    }

    if l3_etymology_count >= 1 && has_l3_pos && !has_l4_pos_under_etymology {
        return HomographPattern::SingleEtymologyWithFlatPos;
    }

    HomographPattern::Other(format!(
        "etym:{} has_l4pos:{} has_l3pos:{}",
        l3_etymology_count, has_l4_pos_under_etymology, has_l3_pos
    ))
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

    let mut pattern_counts: HashMap<HomographPattern, (u32, Vec<String>)> = HashMap::new();
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

                let pattern = classify_english_structure(&page.rev_text);
                pages_with_english += 1;
                
                let entry = pattern_counts.entry(pattern).or_insert((0, Vec::new()));
                entry.0 += 1;
                if args.examples && entry.1.len() < 3 {
                    entry.1.push(page.title);
                }
            }
            None => break,
        }
    }

    let mut sorted: Vec<_> = pattern_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    println!("Homograph Pattern Analysis");
    println!("({} pages scanned, {} with English sections)", pages_processed, pages_with_english);
    println!("==================================================");
    println!();

    for (pattern, (count, examples)) in sorted.iter() {
        let pct = (*count as f64 / pages_with_english as f64) * 100.0;
        println!("{:3}% ({:6} pages) - {:?}", pct as u32, count, pattern);
        if args.examples && !examples.is_empty() {
            println!("               Examples: {}", examples.join(", "));
        }
        println!();
    }

    println!("==================================================");
    println!("Summary:");
    for (pattern, (count, _)) in sorted.iter() {
        let pct = (*count as f64 / pages_with_english as f64) * 100.0;
        println!("  {:<45} {:3}% ({:6} pages)", format!("{:?}", pattern), pct as u32, count);
    }

    Ok(())
}
