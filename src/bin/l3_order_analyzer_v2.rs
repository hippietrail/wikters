use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::regex_reader::RegexReader;
use wikters::string_ops_reader::StringOpsReader;
use wikters::wikitext_splitter::{self, Heading};
use wikters::{PageSource, Opts};

#[derive(Debug, Parser)]
#[command(version, about = "Analyze L3 section ordering using clean structural parsing")]
struct Args {
    /// Limit the number of pages to scan
    #[clap(short, long)]
    limit: Option<u64>,

    /// Language to analyze (default: English)
    #[clap(long, default_value = "English")]
    language: String,

    /// Use regex-based hand-rolled parser
    #[clap(short = 'r', long)]
    handrolled: bool,

    /// Use string-ops hand-rolled parser
    #[clap(short = 's', long)]
    stringops: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum L3Pattern {
    PosOnly,                        // Only L3:POS, no Etymology/Pronunciation
    EtymOnly,                       // L3:Etymology only
    PronOnly,                       // L3:Pronunciation only
    EtymFlatThenPronFlat,          // L3:Etym → L3:Pron (ordered)
    PronFlatThenEtymFlat,          // L3:Pron → L3:Etym (ordered)
    EtymWithNestedPron,            // L3:Etym with L4:Pron inside (no separate L3:Pron)
    PronWithNestedEtym,            // L3:Pron with L4:Etym inside (no separate L3:Etym)
    Other(String),
}

/// Check if a heading text matches a category (case-insensitive prefix)
fn heading_matches(text: &str, category: &str) -> bool {
    text.to_lowercase().starts_with(&category.to_lowercase())
}

fn is_pos(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "noun", "verb", "adjective", "adverb", "preposition", "conjunction",
        "interjection", "determiner", "pronoun", "article", "numeral", "particle",
    ]
    .iter()
    .any(|pos| lower.contains(pos))
}

/// Analyze the L3 section ordering within a language section.
fn classify_l3_pattern(headings: &[Heading], section_start: usize, section_end: usize) -> L3Pattern {
    // Get all L3 headings in this section
    let l3_indices: Vec<usize> = headings[section_start..section_end]
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            if h.level == 3 {
                Some(section_start + i)
            } else {
                None
            }
        })
        .collect();

    if l3_indices.is_empty() {
        return L3Pattern::Other("no_l3".to_string());
    }

    // Categorize each L3 heading
    let mut first_etym_idx = None;
    let mut first_pron_idx = None;
    let mut first_pos_idx = None;

    for &idx in &l3_indices {
        let text = &headings[idx].text;
        if heading_matches(text, "etymology") && first_etym_idx.is_none() {
            first_etym_idx = Some(idx);
        } else if heading_matches(text, "pronunciation") && first_pron_idx.is_none() {
            first_pron_idx = Some(idx);
        } else if is_pos(text) && first_pos_idx.is_none() {
            first_pos_idx = Some(idx);
        }
    }

    match (first_etym_idx, first_pron_idx) {
        (Some(e_idx), Some(p_idx)) => {
            // Both Etymology and Pronunciation exist at L3
            if e_idx < p_idx {
                L3Pattern::EtymFlatThenPronFlat
            } else {
                L3Pattern::PronFlatThenEtymFlat
            }
        }
        (Some(e_idx), None) => {
            // Only Etymology at L3 - check if there's nested Pronunciation (L4 under Etymology)
            let has_nested_pron = has_nested_heading(headings, e_idx, "pronunciation");
            if has_nested_pron {
                L3Pattern::EtymWithNestedPron
            } else {
                L3Pattern::EtymOnly
            }
        }
        (None, Some(p_idx)) => {
            // Only Pronunciation at L3 - check if there's nested Etymology (L4 under Pronunciation)
            let has_nested_etym = has_nested_heading(headings, p_idx, "etymology");
            if has_nested_etym {
                L3Pattern::PronWithNestedEtym
            } else {
                L3Pattern::PronOnly
            }
        }
        (None, None) => {
            // Neither Etymology nor Pronunciation at L3
            if first_pos_idx.is_some() {
                L3Pattern::PosOnly
            } else {
                L3Pattern::Other("no_etym_pron_pos".to_string())
            }
        }
    }
}

/// Check if there's a heading at the given level within the section starting at `section_idx`.
/// Looks for L4+ headings under the given L3 heading until the next L3 or end of parent section.
fn has_nested_heading(headings: &[Heading], section_idx: usize, category: &str) -> bool {
    let section_level = headings[section_idx].level;

    // Look at all headings after this one
    for h in &headings[section_idx + 1..] {
        if h.level <= section_level {
            // Hit a heading at same level or shallower - stop
            break;
        }
        if h.level == section_level + 1 && heading_matches(&h.text, category) {
            // Found a matching nested heading
            return true;
        }
    }
    false
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

    let mut pattern_counts: HashMap<L3Pattern, (u32, Vec<String>)> = HashMap::new();
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

                let (headings, _content) = wikitext_splitter::split_by_headings(&page.rev_text);

                if let Some((lang_start, lang_end)) = wikitext_splitter::find_language_section(&headings, &args.language) {
                    let pattern = classify_l3_pattern(&headings, lang_start, lang_end);
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

    println!("L3 Section Order Pattern Analysis (v2 - structural)");
    println!("Language: {}", args.language);
    println!("({} pages scanned)", pages_processed);
    println!("==================================================");
    println!();

    for (pattern, (count, examples)) in sorted.iter() {
        let pct = (*count as f64 / pages_processed as f64) * 100.0;
        println!("{:3}% ({:6} pages) - {:?}", pct as u32, count, pattern);
        println!("               Examples: {}", examples.join(", "));
        println!();
    }

    Ok(())
}
