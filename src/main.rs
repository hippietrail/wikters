use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use regex::Regex;

mod heading_lists;
use heading_lists::{HEADING_WHITELIST, HEADING_BLACKLIST};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    #[clap(short, long, action = clap::ArgAction::SetTrue)]
    xml: bool,
}

struct HeadingsSeen {
    all: HashMap<String, u64>,      // all headings, how it works now
    white: HashMap<String, u64>,    // headings I specifically want
    grey: HashMap<String, u64>,     // headings I didn't consider, rare headings, mistakes, etc.
    black: HashMap<String, u64>,    // headings I specifically don't want
}

impl HeadingsSeen {
    fn new() -> Self {
        HeadingsSeen {
            all: HashMap::new(),
            white: HashMap::new(),
            grey: HashMap::new(),
            black: HashMap::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let output_xml = args.xml;

    let stdin = io::stdin();
    let mut reader = Reader::from_reader(stdin.lock());

    let mut last_text_content: Option<String> = None;
    let mut ns_key: Option<i32> = None;
    let mut page_title: String = String::new();
    let mut page_ns: Option<i32> = None;
    let mut page_id: Option<i32> = None;
    let mut page_rev_id: Option<i32> = None;
    let mut page_rev_contrib_id: Option<i32> = None;
    let mut page_rev_text: String = String::new();

    let mut buffer = Vec::new();

    let mut page_num: u64 = 0;
    let mut section_num: u64 = 0;

    // a map of strings (of some kind, &str or string or whatever) to u64 to count how many times we've seen each heading
    let mut headings_seen = HeadingsSeen::new();

    if output_xml {
        println!("<wiktionary>");
    }

    let mut just_emitted_update = false;

    loop {
    // while page_num < 3 {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(node)) => match get_node_name(&node).as_str() {
                "namespace" => start_namespace(&node, &mut ns_key, &mut last_text_content),
                "page" => start_page(&mut page_title, &mut page_ns, &mut page_id, &mut page_rev_id, &mut page_rev_text),
                "title" => start_page_title(&mut last_text_content),
                "ns" => start_page_ns(&mut last_text_content, &mut page_ns),
                "id" => start_id(&mut last_text_content),
                "text" => start_page_rev_text(&mut last_text_content),
                _ => {}
            },
            Ok(Event::Empty(node)) => match get_node_name(&node).as_str() {
                "namespace" => {
                    start_namespace(&node, &mut ns_key, &mut last_text_content);
                    end_namespace(ns_key, &last_text_content);
                },
                _ => {}
            },
            Ok(Event::End(node)) => match get_node_name_end(&node).as_str() {
                "namespace" => end_namespace(ns_key, &last_text_content),
                "title" => end_page_title(&mut page_title, &mut last_text_content),
                "ns" => end_page_ns(&mut page_ns, &mut last_text_content),
                "id" => end_id(&mut page_id, &mut page_rev_id, &mut page_rev_contrib_id, &mut last_text_content),
                "text" => end_page_rev_text(&mut page_rev_text, &mut last_text_content),
                "page" => end_page(output_xml, &page_title, page_ns, page_id, page_rev_id, &page_rev_text,
                    &mut page_num, &mut section_num, &mut just_emitted_update, &mut headings_seen),
                _ => {}
            },
            Ok(Event::Text(text)) => {
                let s = String::from_utf8(text.to_vec()).unwrap();
                if let Some(ref mut last_text_content) = last_text_content {
                    last_text_content.push_str(&s);
                } else {
                    last_text_content = Some(s);
                }
            }
            Ok(Event::Eof) => { break }
            Ok(_) => {}
            Err(_error) => break //println!("{}", error),
        }

        // Clear the buffer for the next event
        buffer.clear();
    }

    if !just_emitted_update {
        emit_update(output_xml, &mut headings_seen);
    }

    if output_xml {
        println!("</wiktionary>");
    }

    Ok(())
}

fn end_page(output_xml: bool, title: &String, namespace: Option<i32>, page_id: Option<i32>, rev_id: Option<i32>, text: &String,
        page_num: &mut u64, section_num: &mut u64, just_emitted_update: &mut bool, headings_seen: &mut HeadingsSeen) {
    if namespace.unwrap() == 0 {
        let all_lang_headings_regex = Regex::new(r"(?m)^== ?([^=]*?) ?== *$\n").unwrap();
        let our_lang_headings_regex = Regex::new(r"(?m)^== ?(English|Translingual) ?== *$\n").unwrap();
        let mut lang_headings: Vec<String> = Vec::new();
        let mut languages: Vec<String> = Vec::new();

        for capture in all_lang_headings_regex.captures_iter(text) {
            if let (Some(heading), Some(lang)) = (capture.get(0), capture.get(1)) {
                let heading_string = heading.as_str().to_string();
                let lang_string = lang.as_str().to_string();

                lang_headings.push(heading_string);
                languages.push(lang_string);
            }
        }
        
        languages.retain(|lang| lang == "English" || lang == "Translingual");

        if languages.len() == 0 { return }

        *page_num += 1;

        let mut page_output: String = if output_xml {
            format!("  <p n=\"{}\" pid=\"{}\" rid=\"{}\">\n    <t>{}</t>",
            page_num, page_id.unwrap(), rev_id.unwrap(), title)
        } else {
            format!("{}", title)
        };

        // now split the text by the same regex
        let splitted = our_lang_headings_regex.split(text).collect::<Vec<&str>>();

        let mut sections_output_vec: Vec<String> = Vec::new();

        for (i, section) in splitted.iter().enumerate().skip(1) {
            *section_num += 1;

            let mut section_output: String = if output_xml {
                format!("    <s n=\"{}\" l=\"{}\">", section_num, languages[i-1])
            } else {
                format!("  {}", languages[i-1])
            };
            
            let mut inner_section = *section;

            if let Some(heading) = all_lang_headings_regex.find(section) {
                inner_section = &section[0..heading.start()];
            }
            // let chosen_stuff = inner_section;

            // let mut headings = get_all_headings(inner_section);

            // headings.retain(|heading| !HEADING_BLACKLIST.contains(&heading.0.as_str()));

            let all_headings = get_all_headings(inner_section);

            // headings.retain(|heading| !HEADING_BLACKLIST.contains(&heading.0.as_str()));
            let nonblack_headings = all_headings
                .iter()
                .filter(|heading| !HEADING_BLACKLIST.contains(&heading.0.as_str()))
                .collect::<Vec<_>>();

            // TODO: handle whitelist and keep track of white, grey, and black counts
            let headings = nonblack_headings;

            let depth: i32 = if output_xml { 2 } else { -2 };

            if headings.len() > 0 {
                let chosen_stuff = "\n".to_owned() + &headings
                    .iter()
                    .map(|h| format!("{:width$}{}",
                        "", h.0, width = (h.1 as i32 * 2 + depth) as usize
                    ))
                    .collect::<Vec<String>>()
                    .join("\n");

                // update seen heading count
                // TODO update seen white, grey, and black headings
                for heading in &headings {
                    *headings_seen.all.entry(heading.0.clone()).or_insert(0) += 1;
                }

                if output_xml {
                    section_output += "\n";
                    section_output += &format!("      <x>{}</x>", chosen_stuff);
                    section_output += "\n";
                    section_output += "    </s>";
                } else {
                    section_output += &format!("{}\n", chosen_stuff);
                }

                if chosen_stuff.len() == 0 {
                    eprintln!("** have headings but no stuff chosen **")
                }

                sections_output_vec.push(section_output);
            }
        }

        if sections_output_vec.len() > 0 {
            page_output += "\n";
            page_output += &sections_output_vec.join("\n");
            if output_xml {
                page_output += "\n";
                page_output += "  </p>";
            }
            println!("{}", page_output);

            // every n pages, emit an update
            if *page_num %256 == 0 {
                emit_update(output_xml, headings_seen);
                *just_emitted_update = true;
            } else {
                *just_emitted_update = false;
            }
        }
    }
}

fn get_node_name(node: &quick_xml::events::BytesStart) -> String {
    String::from_utf8(node.name().0.to_vec()).unwrap()
}

fn get_node_name_end(node: &quick_xml::events::BytesEnd) -> String {
    String::from_utf8(node.name().0.to_vec()).unwrap()
}

fn start_namespace(node: &quick_xml::events::BytesStart, ns_key: &mut Option<i32>, last_text_content: &mut Option<String>) {
    if let Some(att) = node.attributes().find(|a| a.as_ref().unwrap().key == quick_xml::name::QName(b"key")) {
        *ns_key = Some(String::from_utf8(att.unwrap().value.to_vec()).unwrap().parse::<i32>().unwrap());
    }
    *last_text_content = None; // Reset for each namespace
}

fn end_namespace(_ns_key: Option<i32>, last_text_content: &Option<String>) {
    // The default namespace, 0, has no name
    let _ns_text = last_text_content.as_ref().unwrap_or(&String::from("")).clone();
    // println!("namespace {} : \"{}\"", ns_key.unwrap(), ns_text);
}

fn start_page(page_title: &mut String, page_ns: &mut Option<i32>,
        page_id: &mut Option<i32>, page_rev_id: &mut Option<i32>, page_rev_text: &mut String) {
    *page_title = String::new();
    *page_ns = None;
    *page_id = None;
    *page_rev_id = None;
    *page_rev_text = String::new();
}

fn start_page_title(last_text_content: &mut Option<String>) {
    *last_text_content = None;
}

fn end_page_title(page_title: &mut String, last_text_content: &mut Option<String>) {
    *page_title = last_text_content.take().unwrap_or_default();
}

fn start_page_ns(last_text_content: &mut Option<String>, page_ns: &mut Option<i32>) {
    *page_ns = None;
    *last_text_content = None;
}

fn end_page_ns(page_ns: &mut Option<i32>, last_text_content: &mut Option<String>) {
    let ns_text = last_text_content.take().unwrap_or_default();
    *page_ns = ns_text.parse::<i32>().ok();
}

fn start_id(last_text_content: &mut Option<String>) {
    *last_text_content = None;
}

fn end_id(page_id: &mut Option<i32>, page_rev_id: &mut Option<i32>, page_rev_contrib_id: &mut Option<i32>, last_text_content: &mut Option<String>) {
    let id = last_text_content.take().unwrap_or_default().parse::<i32>().unwrap();
    if page_id.is_none() {
        *page_id = Some(id);
    } else if page_rev_id.is_none() {
        *page_rev_id = Some(id);
    } else if page_rev_contrib_id.is_none() {
        *page_rev_contrib_id = Some(id);
    }
}

fn start_page_rev_text(last_text_content: &mut Option<String>) {
    *last_text_content = None;
}

fn end_page_rev_text(page_rev_text: &mut String, last_text_content: &mut Option<String>) {
    *page_rev_text = last_text_content.take().unwrap_or_default();
}

fn get_all_headings(inner_section: &str) -> Vec<(String, u8)> {
    let all_headings_regex = Regex::new(r"(?m)^(===+) ?([^=]*?) ?===+ *$\n").unwrap();
    let mut headings: Vec<(String, u8)> = Vec::new();

    for capture in all_headings_regex.captures_iter(inner_section) {
        let heading_depth = capture.get(1).unwrap().as_str().len();
        let heading_name = capture.get(2).unwrap();
        let name_string = heading_name.as_str().to_string();

        headings.push((name_string, heading_depth.try_into().unwrap()));
    }
    headings
}

fn emit_update(output_xml: bool, headings_seen: &mut HeadingsSeen) {
    if output_xml {
        println!("  <update>");
    } else {
        println!("--update--");
    }
    let mut sorted_headings: Vec<_> = headings_seen.all.iter().collect();
    sorted_headings.sort_by(|a, b| 
        b.1.cmp(a.1)
        .then_with(|| a.0.cmp(b.0)));
    
    for (heading, count) in sorted_headings {
        if output_xml {
            println!("    <h n=\"{}\" c=\"{}\"/>", heading, count);
        } else {
            println!("{}: {}", heading, count);
        }
    }
    if output_xml {
        println!("  </update>");
    } else {
        println!("");
    }
}
