use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::regex_reader::RegexReader;
use wikters::string_ops_reader::StringOpsReader;
use wikters::{PageSource, Opts};

#[derive(Debug, Parser)]
#[command(version, about = "Analyze L4 (====) heading patterns under L3 Etymology/Pronunciation")]
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
struct L4Context {
    parent_type: String,  // "Etymology", "Pronunciation", etc
    l4_type: String,      // "Pronunciation", "Etymology", "Noun", etc
}

fn normalize_section_type(text: &str) -> String {
    // Strip numbers from section types for grouping
    // "Etymology 1" -> "Etymology", "Pronunciation 2" -> "Pronunciation", etc
    let mut result = String::new();
    for ch in text.chars() {
        if ch.is_alphabetic() {
            result.push(ch);
        } else if ch == ' ' && !result.is_empty() && result.ends_with(' ') {
            continue; // Skip multiple spaces
        } else if ch == ' ' {
            result.push(ch);
        }
    }
    result.trim().to_string()
}

fn analyze_l4_patterns(text: &str) -> Vec<(L4Context, String)> {
    let lines: Vec<_> = text.lines().collect();
    let (start, end) = match get_english_section(text) {
        Some(range) => range,
        None => return Vec::new(),
    };

    let mut patterns = Vec::new();
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
            last_l3_type = normalize_section_type(&heading_text);
        } else if level == 4 && !last_l3_type.is_empty() {
            let normalized_l4 = normalize_section_type(&heading_text);
            patterns.push((
                L4Context {
                    parent_type: last_l3_type.clone(),
                    l4_type: normalized_l4.clone(),
                },
                format!("==={}===\n===={}====", last_l3_type, normalized_l4),
            ));
        }
    }

    patterns
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

    let mut l4_counts: HashMap<L4Context, (u32, Vec<String>)> = HashMap::new();
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

                let patterns = analyze_l4_patterns(&page.rev_text);
                for (context, _example) in patterns {
                    let entry = l4_counts.entry(context).or_insert((0, Vec::new()));
                    entry.0 += 1;
                    if args.examples && entry.1.len() < 2 {
                        entry.1.push(page.title.clone());
                    }
                }
            }
            None => break,
        }
    }

    // Group by L4 type
    let mut l4_type_totals: HashMap<String, u32> = HashMap::new();
    for (context, (count, _)) in &l4_counts {
        *l4_type_totals.entry(context.l4_type.clone()).or_insert(0) += count;
    }

    // Sort L4 types by frequency
    let mut sorted_types: Vec<_> = l4_type_totals.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));

    println!("L4 Heading Analysis");
    println!("({} pages scanned)", pages_processed);
    println!("==================================================");
    println!();
    println!("Most common L4 heading types:");
    println!();

    for (l4_type, total_count) in sorted_types.iter().take(20) {
        println!("{:30} {:6} occurrences", l4_type, total_count);
    }

    println!();
    println!("==================================================");
    println!("L4 patterns by (L3 parent -> L4 child):");
    println!();

    // Now sort all contexts by frequency
    let mut sorted_contexts: Vec<_> = l4_counts.iter().collect();
    sorted_contexts.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    for (context, (count, examples)) in sorted_contexts.iter().take(40) {
        println!("{:20} -> {:20} {:6}", context.parent_type, context.l4_type, count);
        if args.examples && !examples.is_empty() {
            println!("               Examples: {}", examples.join(", "));
        }
    }

    println!();
    println!("==================================================");
    println!("Total unique (parent -> child) patterns: {}", l4_counts.len());

    Ok(())
}
