use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::wikitext_splitter;
use wikters::PageSource;

#[derive(Debug, Parser)]
#[command(version, about = "Find entries with heading level skips (e.g., === to =====)")]
struct Args {
    /// Limit the number of pages to scan
    #[clap(short, long)]
    limit: Option<u64>,

    /// Show first N examples
    #[clap(short, long, default_value = "5")]
    examples: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let stdin = io::stdin();
    let source = Box::new(QuickXmlReader::new(stdin.lock()));

    let mut source = source;
    let mut pages_processed = 0;
    let mut found_examples = Vec::new();

    loop {
        if let Some(limit) = args.limit {
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

                let (headings, _) = wikitext_splitter::split_by_headings(&page.rev_text);

                if headings.is_empty() {
                    continue;
                }

                // Check for level skips
                let mut has_skip = false;
                for i in 0..headings.len() - 1 {
                    let curr_level = headings[i].level;
                    let next_level = headings[i + 1].level;

                    // Skip if going back up (to same or higher level, which is normal)
                    if next_level <= curr_level {
                        continue;
                    }

                    // Check if skip more than 1 level
                    if next_level > curr_level + 1 {
                        has_skip = true;
                        break;
                    }
                }

                if has_skip && found_examples.len() < args.examples {
                    found_examples.push(page.title.clone());
                }
            }
            None => break,
        }
    }

    println!("Scanned: {} pages", pages_processed);
    println!("Found {} entries with level skips:", found_examples.len());
    for title in found_examples {
        println!("  - {}", title);
    }

    Ok(())
}
