use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::wikitext_splitter::{self, Heading};
use wikters::PageSource;

#[derive(Debug, Parser)]
#[command(version, about = "Show the structural tree of a wiktionary entry")]
struct Args {
    /// Title of the page to find and display
    #[clap(short, long)]
    title: String,

    /// Only show English section (optionally with Translingual)
    #[clap(short, long)]
    main_only: bool,

    /// Include Translingual section with English (only with --main-only)
    #[clap(short, long)]
    with_translingual: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let stdin = io::stdin();
    let source = Box::new(QuickXmlReader::new(stdin.lock()));

    let mut source = source;
    loop {
        match source.next_page()? {
            Some(page) => {
                if page.title == args.title {
                    println!("Found: {}", page.title);
                    println!();

                    let (headings, content_chunks) = wikitext_splitter::split_by_headings(&page.rev_text);

                    if args.main_only {
                        // Show only English (and optionally Translingual)
                        let mut sections_shown = false;

                        for (i, heading) in headings.iter().enumerate() {
                            if heading.level != 2 {
                                continue;
                            }

                            let show = heading.text == "English"
                                || (args.with_translingual && heading.text == "Translingual");

                            if !show {
                                continue;
                            }

                            if sections_shown {
                                println!();
                            }

                            println!("{}:", heading.text);
                            println!("==================================================");

                            // Find next L2 section
                            let next_l2 = headings[i + 1..]
                                .iter()
                                .position(|h| h.level == 2)
                                .map(|p| p + i + 1)
                                .unwrap_or(headings.len());

                            // Show this section's headings
                            for j in (i + 1)..next_l2 {
                                let h = &headings[j];
                                let indent = if h.level >= 2 { h.level - 2 } else { 0 };
                                println!("{}{}", "  ".repeat(indent), h);
                            }

                            sections_shown = true;
                        }
                    } else {
                        // Show full structure
                        println!("Full structure ({} headings):", headings.len());
                        println!("==================================================");

                        for heading in headings.iter() {
                            let indent = if heading.level >= 2 {
                                heading.level - 2
                            } else {
                                0
                            };
                            println!("{}{}", "  ".repeat(indent), heading);
                        }
                    }

                    return Ok(());
                }
            }
            None => {
                println!("Entry not found: {}", args.title);
                return Ok(());
            }
        }
    }
}
