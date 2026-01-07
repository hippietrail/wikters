use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::{PageSource, Opts};

#[derive(Debug, Parser)]
#[command(version, about = "Dump raw English sections for manual inspection")]
struct Args {
    /// Limit the number of pages to scan
    #[clap(short, long)]
    limit: Option<u64>,

    /// Show first N pages with matching language sections (shows raw wikitext)
    #[clap(short, long, default_value = "20")]
    pages_to_show: u64,

    /// Language section to extract (default: English)
    #[clap(long, default_value = "English")]
    language: String,

    /// Filter by title substring (case-insensitive)
    #[clap(long)]
    title_filter: Option<String>,
}

fn get_language_section(text: &str, language: &str) -> Option<String> {
    let lines: Vec<_> = text.lines().collect();
    
    // Find ==Language==
    let start = lines.iter().position(|line| {
        let trimmed = line.trim();
        let lang_heading = format!("=={language}==");
        trimmed == lang_heading || trimmed.starts_with(&lang_heading)
    })?;

    // Find next L2 heading (==SomeLanguage==)
    let end = lines[start + 1..]
        .iter()
        .position(|line| {
            let trimmed = line.trim();
            let leading = trimmed.chars().take_while(|c| *c == '=').count();
            let trailing = trimmed.chars().rev().take_while(|c| *c == '=').count();
            leading == 2 && leading == trailing && leading * 2 < trimmed.len()
        })
        .map(|p| p + start + 1)
        .unwrap_or(lines.len());

    Some(lines[start..end].join("\n"))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let opts = Opts {
        limit: args.limit,
        xml: false,
        no_updates: false,
        sample_rate: None,
        handrolled: false,
    };

    let stdin = io::stdin();
    let source = QuickXmlReader::new(stdin.lock());

    let mut shown = 0;
    let mut scanned = 0;

    for page_result in std::iter::from_fn({
        let mut source = source;
        move || source.next_page().ok().flatten()
    }) {
        scanned += 1;

        if let Some(limit) = opts.limit {
            if scanned > limit {
                break;
            }
        }

        if shown >= args.pages_to_show {
            break;
        }

        // Skip non-article namespace
        if page_result.ns.unwrap_or(-1) != 0 {
            continue;
        }

        // Apply title filter if specified
        if let Some(ref filter) = args.title_filter {
            if !page_result.title.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }
        }

        if let Some(language_section) = get_language_section(&page_result.rev_text, &args.language) {
            shown += 1;
            println!("=== {} ===", page_result.title);
            println!("{}", language_section);
            println!();
        }
    }

    eprintln!("Showed {} pages with English sections (scanned {} pages total)", shown, scanned);

    Ok(())
}
