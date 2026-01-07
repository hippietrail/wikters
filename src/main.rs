use std::error::Error;
use std::io;

use clap::Parser;
use quick_xml::Reader;

// how to access handrolled/process_dump?
use wikters::handrolled::process_dump;
use wikters::qx_iterate;
use wikters::Opts;
use wikters::Page;
use wikters::State;

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

    let mut state = State {
        last_text_content: None,
        ns_key: None,
        page: Page::new(),
        page_num: 0,
        section_num: 0,
        just_emitted_update: false,
    };

    // Choose parsing method based on command line argument
    if args.handrolled {
        // Use hand-rolled parser
        process_dump(&opts, stdin.lock())?;
    } else {
        // Use quick-xml parser
        let mut qx_reader = Reader::from_reader(stdin.lock());
        let mut qx_buffer = Vec::new();

        while args.limit.is_none_or(|limit| state.page_num < limit) {
            if !qx_iterate(&opts, &mut qx_reader, &mut qx_buffer, &mut state) {
                break;
            }
        }
    }

    Ok(())
}
