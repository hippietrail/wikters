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

    /// Show content preview (first N chars under each heading)
    #[clap(short, long, default_value = "0")]
    preview: usize,
}

fn print_tree(headings: &[Heading], content_chunks: &[String], start: usize, end: usize, indent: usize) {
    for i in start..end {
        let heading = &headings[i];
        let prefix = "  ".repeat(indent);
        let content = &content_chunks[i + 1];
        
        let preview = if content.is_empty() {
            String::new()
        } else {
            let trimmed = content.trim();
            let first_line = trimmed.lines().next().unwrap_or("");
            if first_line.len() > 50 {
                format!(" → \"{}...\"", &first_line[..50])
            } else {
                format!(" → \"{}\"", first_line)
            }
        };
        
        println!("{}{}{}", prefix, heading, preview);
    }
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
                    println!("Namespace: {}", page.ns.unwrap_or(-1));
                    println!();

                    let (headings, content_chunks) = wikitext_splitter::split_by_headings(&page.rev_text);

                    println!("Full structure ({} headings):", headings.len());
                    println!("==================================================");
                    
                    // Show all headings with their nesting
                    for (i, heading) in headings.iter().enumerate() {
                        let indent = if heading.level >= 2 { heading.level - 2 } else { 0 };
                        let content = &content_chunks[i + 1];
                        
                        let preview = if args.preview > 0 && !content.is_empty() {
                            let trimmed = content.trim();
                            let first_line = trimmed.lines().next().unwrap_or("");
                            if first_line.is_empty() {
                                String::new()
                            } else {
                                let preview_len = args.preview.min(first_line.len());
                                format!(" → \"{}\"", &first_line[..preview_len])
                            }
                        } else {
                            String::new()
                        };
                        
                        println!("{}{}{}", "  ".repeat(indent), heading, preview);
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
