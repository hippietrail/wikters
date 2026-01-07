use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::regex_reader::RegexReader;
use wikters::string_ops_reader::StringOpsReader;
use wikters::{PageSource, Opts};

#[derive(Debug, Parser)]
#[command(version, about = "Analyze L3 section ordering (Etymology vs Pronunciation ordering)")]
struct Args {
    /// Limit the number of pages to scan
    #[clap(short, long)]
    limit: Option<u64>,

    /// Language to analyze (default: English)
    #[clap(long, default_value = "English")]
    language: String,

    /// Also include Translingual sections
    #[clap(long)]
    with_translingual: bool,

    /// Use regex-based hand-rolled parser
    #[clap(short = 'r', long)]
    handrolled: bool,

    /// Use string-ops hand-rolled parser
    #[clap(short = 's', long)]
    stringops: bool,

    /// Store examples to markdown file (use - for stdout)
    #[clap(long)]
    output_examples: Option<String>,
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

fn get_language_section(text: &str, language: &str) -> Option<(usize, usize)> {
    let lines: Vec<_> = text.lines().collect();
    
    let start = lines.iter().position(|line| {
        let trimmed = line.trim();
        is_valid_heading(trimmed) && 
        count_leading_equals(trimmed) == 2 &&
        trimmed.contains(language)
    })?;

    let end = lines[start + 1..]
        .iter()
        .position(|line| {
            let trimmed = line.trim();
            is_valid_heading(trimmed) && count_leading_equals(trimmed) == 2
        })
        .map(|p| p + start + 1)
        .unwrap_or(lines.len());

    Some((start, end))
}

fn is_etymology_section(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.starts_with("etymology")
}

fn is_pronunciation_section(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.starts_with("pronunciation")
}

fn is_pos_section(text: &str) -> bool {
    let lower = text.to_lowercase();
    ["noun", "verb", "adjective", "adverb", "preposition", "conjunction",
     "interjection", "determiner", "pronoun", "article", "numeral", "particle"]
        .iter()
        .any(|pos| lower.contains(pos))
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum OrderPattern {
    EtymFlatThenPronFlat,          // L3:Etym → L3:Pron (both sequential)
    PronFlatThenEtymFlat,          // L3:Pron → L3:Etym (both sequential)
    EtymFlatThenPronNested,        // L3:Etym → L3:Pron with L4:Pron under Etym
    PronFlatThenEtymNested,        // L3:Pron → L3:Etym with L4:Pron or L4:Etym under Pron
    EtymWithNestedPron,            // L3:Etym with L4:Pron inside (no L3:Pron)
    PronWithNestedEtym,            // L3:Pron with L4:Etym inside (no L3:Etym)
    EtymOnly,                      // L3:Etym only
    PronOnly,                      // L3:Pron only
    PosOnly,                       // Only L3:POS
    Other(String),
}

fn has_nested_l4(lines: &[&str], l3_start: usize, l3_end: usize, section_type: &str) -> bool {
    for i in l3_start + 1..l3_end {
        let trimmed = lines[i].trim();
        if !is_valid_heading(trimmed) {
            continue;
        }
        let level = count_leading_equals(trimmed);
        if level == 3 {
            break; // Next L3 section
        }
        if level == 4 {
            let heading_text = get_heading_text(lines[i]);
            if section_type == "Pronunciation" && is_pronunciation_section(&heading_text) {
                return true;
            }
            if section_type == "Etymology" && is_etymology_section(&heading_text) {
                return true;
            }
            // Check for POS sections nested under Etymology - this indicates homographs
            if section_type == "POS" && is_pos_section(&heading_text) {
                return true;
            }
        }
    }
    false
}

fn get_l3_order_pattern(text: &str, language: &str) -> OrderPattern {
    let lines: Vec<_> = text.lines().collect();
    let (start, end) = match get_language_section(text, language) {
        Some(range) => range,
        None => return OrderPattern::Other(format!("no_{}", language.to_lowercase())),
    };

    let mut l3_sections: Vec<(usize, usize, String)> = Vec::new(); // (line_start, line_end, text)

    for i in start + 1..end {
        let trimmed = lines[i].trim();
        
        if !is_valid_heading(trimmed) || count_leading_equals(trimmed) != 3 {
            continue;
        }

        let heading_text = get_heading_text(lines[i]);
        l3_sections.push((i, 0, heading_text)); // end calculated below
    }

    if l3_sections.is_empty() {
        return OrderPattern::Other("no_l3".to_string());
    }

    // Calculate end line for each L3 section
    for i in 0..l3_sections.len() {
        let next_l3_line = if i + 1 < l3_sections.len() {
            l3_sections[i + 1].0
        } else {
            end
        };
        l3_sections[i].1 = next_l3_line;
    }

    let mut etymology_idx = None;
    let mut pronunciation_idx = None;
    let mut pos_idx = None;

    for (idx, (_, _, text)) in l3_sections.iter().enumerate() {
        if is_etymology_section(text) && etymology_idx.is_none() {
            etymology_idx = Some(idx);
        }
        if is_pronunciation_section(text) && pronunciation_idx.is_none() {
            pronunciation_idx = Some(idx);
        }
        if is_pos_section(text) && pos_idx.is_none() {
            pos_idx = Some(idx);
        }
    }

    match (etymology_idx, pronunciation_idx) {
        (Some(e), Some(p)) => {
            // Both exist at L3 - check for nesting
            let (etym_start, etym_end, _) = l3_sections[e];
            let (pron_start, pron_end, _) = l3_sections[p];
            
            let etym_has_nested_pron = has_nested_l4(&lines, etym_start, etym_end, "Pronunciation");
            let pron_has_nested_etym = has_nested_l4(&lines, pron_start, pron_end, "Etymology");
            
            if e < p {
                // Etymology before Pronunciation
                if etym_has_nested_pron {
                    OrderPattern::EtymFlatThenPronNested
                } else {
                    OrderPattern::EtymFlatThenPronFlat
                }
            } else {
                // Pronunciation before Etymology
                if pron_has_nested_etym {
                    OrderPattern::PronFlatThenEtymNested
                } else {
                    OrderPattern::PronFlatThenEtymFlat
                }
            }
        }
        (Some(e), None) => {
            // Only Etymology at L3 - check if it has nested POS (homographs with multiple senses)
            let (etym_start, etym_end, _) = l3_sections[e];
            let etym_has_nested_pos = has_nested_l4(&lines, etym_start, etym_end, "POS");
            if etym_has_nested_pos {
                OrderPattern::EtymWithNestedPron // Indicates homograph structure
            } else {
                OrderPattern::EtymOnly
            }
        }
        (None, Some(p)) => {
            // Only Pronunciation at L3
            let (pron_start, pron_end, _) = l3_sections[p];
            let pron_has_nested_etym = has_nested_l4(&lines, pron_start, pron_end, "Etymology");
            if pron_has_nested_etym {
                OrderPattern::PronWithNestedEtym
            } else {
                OrderPattern::PronOnly
            }
        }
        (None, None) => {
            // Neither Etymology nor Pronunciation
            if pos_idx.is_some() {
                OrderPattern::PosOnly
            } else {
                OrderPattern::Other("no_etym_pron_pos".to_string())
            }
        }
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

    let mut pattern_counts: HashMap<OrderPattern, (u32, Vec<String>)> = HashMap::new();
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

                let mut patterns_found = vec![];
                
                // Analyze requested language
                patterns_found.push(get_l3_order_pattern(&page.rev_text, &args.language));
                
                // Optionally analyze Translingual
                if args.with_translingual {
                    patterns_found.push(get_l3_order_pattern(&page.rev_text, "Translingual"));
                }
                
                for pattern in patterns_found {
                    let entry = pattern_counts.entry(pattern).or_insert((0, Vec::new()));
                    entry.0 += 1;
                    if entry.1.len() < 4 {
                        entry.1.push(page.title.clone());
                    }
                }
            }
            None => break,
        }
    }

    let mut sorted: Vec<_> = pattern_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    println!("L3 Section Order Pattern Analysis");
    println!("Language: {}{}", args.language, if args.with_translingual { " + Translingual" } else { "" });
    println!("({} pages scanned)", pages_processed);
    println!("==================================================");
    println!();

    for (pattern, (count, examples)) in sorted.iter() {
        let pct = (*count as f64 / pages_processed as f64) * 100.0;
        println!("{:3}% ({:6} pages) - {:?}", pct as u32, count, pattern);
        println!("               Examples: {}", examples.join(", "));
        println!();
    }

    // Output examples if requested
    if let Some(output) = args.output_examples {
        let mut content = String::new();
        content.push_str("# L3 Order Pattern Examples\n\n");
        
        for (pattern, (count, examples)) in &pattern_counts {
            content.push_str(&format!("## {:?}\n", pattern));
            content.push_str(&format!("Count: {}\n", count));
            content.push_str(&format!("Examples: {}\n\n", examples.join(", ")));
        }

        if output == "-" {
            println!("\n==================================================\nExamples:");
            println!("{}", content);
        } else {
            std::fs::write(&output, content)?;
            eprintln!("Wrote examples to {}", output);
        }
    }

    Ok(())
}
