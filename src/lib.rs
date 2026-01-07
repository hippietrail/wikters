use std::collections::HashMap;
use std::io::StdinLock;

use quick_xml::{
    events::{BytesStart, Event},
    name::QName,
    reader::Reader,
};
use regex::Regex;

mod heading_and_template_lists;
use heading_and_template_lists::{HEADING_BLACKLIST, HEADING_WHITELIST};
use heading_and_template_lists::{TEMPLATE_BLACKLIST, TEMPLATE_WHITELIST};

pub mod handrolled;

#[derive(Debug)]
pub struct Opts {
    pub limit: Option<u64>,
    pub xml: bool,
    pub no_updates: bool,
    pub sample_rate: Option<u64>,
    pub handrolled: bool,
}

// Type aliases for complex tuple types
type Heading = (String, u8);
type Template = (String, u16);
type HeadingVec = Vec<Heading>;
type TemplateVec = Vec<Template>;

pub struct Page {
    title: String,
    ns: Option<i32>,
    id: Option<i32>,
    rev_id: Option<i32>,
    rev_contrib_id: Option<i32>,
    rev_text: String,
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

pub struct State {
    pub last_text_content: Option<String>,
    pub ns_key: Option<i32>,
    pub page: Page,

    pub page_num: u64,
    pub section_num: u64,

    pub just_emitted_update: bool,
}

// Called with nothing quick-xml specific when each </page> closing tag has been read

fn parse_page_wikitext(
    page: &Page,           // page's data
    page_num: &mut u64,    // count of chosen pages
    section_num: &mut u64, // count of chosen sections (each page may have English, Translingual, or both)
) {
    if page.ns.unwrap() != 0 {
        return;
    }

    let all_lang_headings_regex = Regex::new(r"(?m)^== ?([^=]*?) ?== *$\n").unwrap();
    let our_lang_headings_regex = Regex::new(r"(?m)^== ?(English|Translingual) ?== *$\n").unwrap();
    let mut lang_headings: Vec<String> = Vec::new();
    let mut languages: Vec<String> = Vec::new();

    for capture in all_lang_headings_regex.captures_iter(&page.rev_text) {
        if let (Some(heading), Some(lang)) = (capture.get(0), capture.get(1)) {
            lang_headings.push(heading.as_str().to_string());
            languages.push(lang.as_str().to_string());
        }
    }

    languages.retain(|lang| lang == "English" || lang == "Translingual");

    if languages.is_empty() {
        return;
    }

    // only count pages we don't reject
    *page_num += 1;

    // now split the text by the same regex
    let split_page_text = our_lang_headings_regex.split(&page.rev_text).collect::<Vec<&str>>();

    let mut lang_sections_output_vec: Vec<String> = Vec::new();

    // skip the prologue before the first heading, usually contains {{also}}
    for (i, lang_sec_text) in split_page_text.iter().enumerate().skip(1) {
        *section_num += 1;

        let mut lang_section_output = languages[i - 1].clone();

        // get everything after this heading
        let mut lang_sec_text = *lang_sec_text;
        // but keep only up to the next heading
        if let Some(heading) = all_lang_headings_regex.find(lang_sec_text) {
            lang_sec_text = &lang_sec_text[0..heading.start()];
        }

        let all_headings_regex = Regex::new(r"(?m)^==(?:=+) ?([^=]*?) ?==(?:=+) *$\n").unwrap();
        let our_headings_regex = Regex::new(r"(?m)^==(?:=+) ?(Noun) ?==(?:=+) *$\n").unwrap();
        let mut headings: Vec<String> = Vec::new();
        let mut heading_names: Vec<String> = Vec::new();

        for capture in all_headings_regex.captures_iter(lang_sec_text) {
            if let (Some(heading), Some(heading_name)) = (capture.get(0), capture.get(1)) {
                headings.push(heading.as_str().to_string());
                heading_names.push(heading_name.as_str().to_string());
            }
        }

        heading_names.retain(|heading_name| heading_name == "Noun");

        if heading_names.is_empty() {
            continue;
        }

        let split_section_text = our_headings_regex.split(&lang_sec_text).collect::<Vec<&str>>();

        let mut heading_sections_output_vec: Vec<String> = Vec::new();

        for (j, section_text) in split_section_text.iter().enumerate().skip(1) {
            // let lump = section_text.replace("\n", "\\n").chars().take(72).collect::<String>();
            // let's find 'lump' a different way: let's iterate through the lines in section_text
            // and the first line to begin with { is the lump
            let mut lump = String::new();
            for line in section_text.lines() {
                if line.starts_with("{{en-") || line.starts_with("{{head|en|") || line.starts_with("{{head|mul|") {
                    lump = line.to_string();
                    break;
                }
            }
            println!("{}\t{}\t{}\t{}\t{}{}",
                page.title,
                languages[i - 1],
                j,
                if j == 0 { "⏺" } else { &heading_names[j - 1] },
                if j == 0 { "⏺" } else { "" },
                lump);
        }
    }

    // let mut page_output = page.title.clone();

    // if !lang_sections_output_vec.is_empty() {
    //     page_output += "\n";
    //     page_output += &lang_sections_output_vec.join("\n");
    //     println!("<<<<{}>>>>", page_output);
    // }
}

/////////////// quick-xml stuff ///////////

// Does one 'iteration' of the quick-xml loop.
// This does not mean get the next page.
// In the quick-xml case it means one 'Event'
// Calls `end_page` when it gets to the </page> - calls with nothing quick-xml specific!

pub fn qx_iterate(
    opts: &Opts,
    qx_reader: &mut Reader<StdinLock<'static>>,
    qx_buffer: &mut Vec<u8>,
    state: &mut State,
) -> bool {
    match qx_reader.read_event_into(qx_buffer) {
        Ok(Event::Start(node)) => match node.name().as_ref() {
            b"namespace" => qx_start_namespace(&node, &mut state.ns_key, &mut state.last_text_content),
            b"page" => qx_start_page(&mut state.page),
            b"title" => qx_start_page_title(&mut state.last_text_content),
            b"ns" => qx_start_page_ns(&mut state.last_text_content, &mut state.page.ns),
            b"id" => qx_start_id(&mut state.last_text_content),
            b"text" => qx_start_page_rev_text(&mut state.last_text_content),
            _ => {}
        },
        Ok(Event::Empty(node)) => {
            if node.name().as_ref() == b"namespace" {
                qx_start_namespace(&node, &mut state.ns_key, &mut state.last_text_content);
                qx_end_namespace(state.ns_key, &state.last_text_content);
            }
        }
        Ok(Event::End(node)) => match node.name().as_ref() {
            b"namespace" => qx_end_namespace(state.ns_key, &state.last_text_content),
            b"title" => qx_end_page_title(&mut state.page.title, &mut state.last_text_content),
            b"ns" => qx_end_page_ns(&mut state.page.ns, &mut state.last_text_content),
            b"id" => qx_end_id(&mut state.page, &mut state.last_text_content),
            b"text" => qx_end_page_rev_text(&mut state.page.rev_text, &mut state.last_text_content),
            b"page" => qx_end_page(opts, state),
            _ => {}
        },
        Ok(Event::Text(text)) => {
            let s = String::from_utf8(text.to_vec()).unwrap();
            if let Some(ref mut last_text_content) = state.last_text_content {
                last_text_content.push_str(&s);
            } else {
                state.last_text_content = Some(s);
            }
        }
        Ok(Event::Eof) => return false,
        Ok(_) => {}
        Err(_error) => return false,
    }

    // Clear the buffer for the next event
    qx_buffer.clear();
    true
}

///// quick-xml implementation functions moved from main part of code

// siteinfo/namespaces/namespace
fn qx_start_namespace(node: &BytesStart, ns_key: &mut Option<i32>, last_text_content: &mut Option<String>) {
    if let Some(att) = node.attributes().find(|a| a.as_ref().unwrap().key == QName(b"key")) {
        *ns_key = Some(
            String::from_utf8(att.unwrap().value.to_vec())
                .unwrap()
                .parse::<i32>()
                .unwrap(),
        );
    }
    *last_text_content = None; // Reset for each namespace
}

// siteinfo/namespaces/namespace
fn qx_end_namespace(_ns_key: Option<i32>, last_text_content: &Option<String>) {
    // The default namespace, 0, has no name
    let _ns_text = last_text_content.as_ref().unwrap_or(&String::from("")).clone();
}

fn qx_start_page(page: &mut Page) {
    *page = Page::new();
}

fn qx_end_page(opts: &Opts, state: &mut State) {
    parse_page_wikitext(&state.page, &mut state.page_num, &mut state.section_num);
}

fn qx_start_page_title(last_text_content: &mut Option<String>) {
    *last_text_content = None;
}

fn qx_end_page_title(page_title: &mut String, last_text_content: &mut Option<String>) {
    *page_title = last_text_content.take().unwrap_or_default();
}

fn qx_start_page_ns(last_text_content: &mut Option<String>, page_ns: &mut Option<i32>) {
    *page_ns = None;
    *last_text_content = None;
}

fn qx_end_page_ns(page_ns: &mut Option<i32>, last_text_content: &mut Option<String>) {
    let ns_text = last_text_content.take().unwrap_or_default();
    *page_ns = ns_text.parse::<i32>().ok();
}

fn qx_start_id(last_text_content: &mut Option<String>) {
    *last_text_content = None;
}

fn qx_end_id(page: &mut Page, last_text_content: &mut Option<String>) {
    let id = last_text_content.take().unwrap_or_default().parse::<i32>().unwrap();
    if page.id.is_none() {
        page.id = Some(id);
    } else if page.rev_id.is_none() {
        page.rev_id = Some(id);
    } else if page.rev_contrib_id.is_none() {
        page.rev_contrib_id = Some(id);
    }
}

fn qx_start_page_rev_text(last_text_content: &mut Option<String>) {
    *last_text_content = None;
}

fn qx_end_page_rev_text(page_rev_text: &mut String, last_text_content: &mut Option<String>) {
    *page_rev_text = last_text_content.take().unwrap_or_default();
}
