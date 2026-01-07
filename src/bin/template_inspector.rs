use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;

use wikters::quick_xml_reader::QuickXmlReader;
use wikters::regex_reader::RegexReader;
use wikters::string_ops_reader::StringOpsReader;
use wikters::{PageSource, Opts};

#[derive(Debug, Parser)]
#[command(version, about = "Analyze template usage in Wiktionary dump")]
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

    /// Show only templates with at least this many occurrences
    #[clap(long, default_value = "1")]
    min_count: u32,

    /// Show detailed variants (not just template names)
    #[clap(long)]
    verbose: bool,
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

    let mut template_counts: HashMap<String, u32> = HashMap::new();
    let mut template_variants: HashMap<String, Vec<String>> = HashMap::new();
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

                // Extract templates from the text
                for line in page.rev_text.lines() {
                    // Only process lines that look like template definitions or POS sections
                    if !line.contains("{{") {
                        continue;
                    }

                    // Look for template starts at the beginning of lines (ignoring whitespace)
                    if let Some(start) = line.find("{{") {
                        let before_template = &line[0..start];
                        // Only count if the line starts with the template (possibly with whitespace)
                        if !before_template.trim().is_empty() {
                            continue;
                        }

                        // Extract template name (up to | or }})
                        let after_braces = &line[start + 2..];
                        let end_pos = after_braces
                            .find("|")
                            .unwrap_or_else(|| after_braces.find("}}").unwrap_or(after_braces.len()));

                        let template_name = after_braces[0..end_pos].trim().to_string();

                        // Skip empty names
                        if template_name.is_empty() {
                            continue;
                        }

                        *template_counts.entry(template_name.clone()).or_insert(0) += 1;

                        if args.verbose {
                            // Store first occurrence for variant analysis
                            let template_end = line[start..]
                                .find("}}")
                                .map(|e| start + e + 2)
                                .unwrap_or(line.len());
                            let full_template = line[start..template_end].to_string();
                            template_variants
                                .entry(template_name)
                                .or_insert_with(Vec::new)
                                .push(full_template);
                        }
                    }
                }
            }
            None => break,
        }
    }

    // Sort by count (descending)
    let mut sorted: Vec<_> = template_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    println!("Template Usage Report ({}  pages scanned)", pages_processed);
    println!("================================================");
    println!();

    for (template, count) in sorted {
        if *count < args.min_count {
            continue;
        }

        println!("{:30} {:6}", template, count);

        if args.verbose {
            if let Some(variants) = template_variants.get(template) {
                // Show unique variants
                let unique: std::collections::HashSet<_> = variants.iter().cloned().collect();
                let mut variant_list: Vec<_> = unique.iter().cloned().collect();
                variant_list.sort();
                for variant in variant_list.iter().take(5) {
                    println!("  {}", variant);
                }
                if unique.len() > 5 {
                    println!("  ... and {} more variants", unique.len() - 5);
                }
            }
        }
        println!();
    }

    println!("================================================");
    println!("Total unique templates: {}", template_counts.len());

    Ok(())
}
