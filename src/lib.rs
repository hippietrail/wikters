use std::error::Error;

mod heading_and_template_lists;

pub mod regex_reader;
pub mod quick_xml_reader;
pub mod wikitext_parser;

/// Trait for XML dump readers - produces pages from MediaWiki XML
pub trait PageSource {
    fn next_page(&mut self) -> Result<Option<Page>, Box<dyn Error>>;
}

#[derive(Debug)]
pub struct Opts {
    pub limit: Option<u64>,
    pub xml: bool,
    pub no_updates: bool,
    pub sample_rate: Option<u64>,
    pub handrolled: bool,
}

/// Process pages from a PageSource, applying wikitext parsing to each
pub fn process_pages(opts: &Opts, mut source: Box<dyn PageSource>) -> Result<(), Box<dyn Error>> {
    let mut page_num = 0;
    let mut section_num = 0;
    
    loop {
        if let Some(limit) = opts.limit {
            if page_num >= limit {
                break;
            }
        }
        
        match source.next_page()? {
            Some(page) => {
                wikitext_parser::parse_page_wikitext(&page, &mut page_num, &mut section_num);
            }
            None => break,
        }
    }
    
    Ok(())
}



pub struct Page {
    pub title: String,
    pub ns: Option<i32>,
    pub id: Option<i32>,
    pub rev_id: Option<i32>,
    pub rev_contrib_id: Option<i32>,
    pub rev_text: String,
}

impl Page {
    pub fn new() -> Self {
        Page {
            title: String::new(),
            ns: None,
            id: None,
            rev_id: None,
            rev_contrib_id: None,
            rev_text: String::new(),
        }
    }
}




