use std::error::Error;
use std::io;

use clap::Parser;

use wikters::regex_reader::RegexReader;
use wikters::quick_xml_reader::QuickXmlReader;
use wikters::process_pages;
use wikters::Opts;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Args {
    /// Limit the number of pages output.
    #[clap(short, long)]
    pub limit: Option<u64>,

    /// Output in lightweight XML format.
    #[clap(short, long)]
    pub xml: bool,

    /// No updates.
    #[clap(short, long)]
    pub no_updates: bool,

    /// Sample rate. Randomly pick an entry to include with a 1/n chance.
    #[clap(short, long)]
    pub sample_rate: Option<u64>,

    /// Use hand-rolled parser instead of quick-xml.
    #[clap(short = 'r', long)]
    pub handrolled: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let opts = Opts {
        limit: args.limit,
        xml: args.xml,
        no_updates: args.no_updates,
        sample_rate: args.sample_rate,
        handrolled: args.handrolled,
    };

    let stdin = io::stdin();

    // Choose reader implementation based on command line argument
    let source: Box<dyn wikters::PageSource> = if args.handrolled {
        Box::new(RegexReader::new(stdin.lock()))
    } else {
        Box::new(QuickXmlReader::new(stdin.lock()))
    };

    process_pages(&opts, source)?;

    Ok(())
}
