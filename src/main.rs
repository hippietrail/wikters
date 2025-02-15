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
    #[clap(short, long)]
    limit: Option<u64>,
}

struct HeadingsSeen {
    white: HashMap<String, u64>,    // headings I specifically want
    grey: HashMap<String, u64>,     // headings I didn't consider, rare headings, mistakes, etc.
    black: HashMap<String, u64>,    // headings I specifically don't want
}

impl HeadingsSeen {
    fn new() -> Self {
        HeadingsSeen {
            white: HashMap::new(),
            grey: HashMap::new(),
            black: HashMap::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

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

    let mut headings_seen = HeadingsSeen::new();

    if args.xml {
        println!("<wiktionary>");
    }

    let mut just_emitted_update = false;

    while args.limit.map_or(true, |limit| page_num < limit) {
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
                "page" => end_page(args.xml, &page_title, page_ns, page_id, page_rev_id, &page_rev_text,
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
        emit_update(args.xml, &mut headings_seen);
    }

    if args.xml {
        println!("</wiktionary>");
    }

    Ok(())
}

fn end_page(
    output_xml: bool,                // output format
    title: &String,                  // page's title from the dump
    namespace: Option<i32>,          // page's namespace
    page_id: Option<i32>,            // page's id
    rev_id: Option<i32>,             // page's revision id (history dumps have many, our dumps have one)
    pagetext: &String,                   // page text
    page_num: &mut u64,              // count of chosen pages
    section_num: &mut u64,           // count of chosen sections (each page may have English, Translingual, or both)
    just_emitted_update: &mut bool,  // flag so we don't emit the final update if we just emitted one
    headings_seen: &mut HeadingsSeen // we count how many times we see each heading
) {
    if namespace.unwrap() == 0 {
        let all_lang_headings_regex = Regex::new(r"(?m)^== ?([^=]*?) ?== *$\n").unwrap();
        let our_lang_headings_regex = Regex::new(r"(?m)^== ?(English|Translingual) ?== *$\n").unwrap();
        let mut lang_headings: Vec<String> = Vec::new();
        let mut languages: Vec<String> = Vec::new();

        for capture in all_lang_headings_regex.captures_iter(pagetext) {
            if let (Some(heading), Some(lang)) = (capture.get(0), capture.get(1)) {
                lang_headings.push(heading.as_str().to_string());
                languages.push(lang.as_str().to_string());
            }
        }

        languages.retain(|lang| lang == "English" || lang == "Translingual");

        if languages.len() == 0 { return }

        // only count pages we don't reject
        *page_num += 1;

        let mut page_output = if output_xml {
            format!("  <p n=\"{}\" pid=\"{}\" rid=\"{}\">\n    <t>{}</t>",
                page_num, page_id.unwrap(), rev_id.unwrap(), title)
        } else {
            title.clone()
        };

        // now split the text by the same regex
        let split_pagetext = our_lang_headings_regex.split(pagetext).collect::<Vec<&str>>();

        let mut sections_output_vec: Vec<String> = Vec::new();

        // skip the prologue before the first heading, usually contains {{also}}
        for (i, langsectext) in split_pagetext.iter().enumerate().skip(1) {
            *section_num += 1;

            let mut section_output = match output_xml {
                true => format!("    <s n=\"{}\" l=\"{}\">", section_num, languages[i-1]),
                false => format!("  {}", languages[i-1]),
            };

            // get everything after this heading
            let mut langsectext = *langsectext;
            // but keep only up to the next heading
            if let Some(heading) = all_lang_headings_regex.find(langsectext) {
                langsectext = &langsectext[0..heading.start()];
            }

            // let nonblack_headings = categorize_and_count(headings_seen, get_headings_and_templates(langsectext));
            let (headings, templates) = get_headings_and_templates(langsectext);
            let nonblack_headings = categorize_and_count(headings_seen, headings);

            if nonblack_headings.len() > 0 {
                let depth = output_xml as i32 * 4 - 2;

                let chosen_headings = "\n".to_owned() + &nonblack_headings
                    .iter()
                    .map(|h| format!("{:width$}{}",
                        "", h.0, width = (h.1 as i32 * 2 + depth) as usize
                    ))
                    .collect::<Vec<String>>()
                    .join("\n");

                if output_xml {
                    section_output += "\n";
                    section_output += &format!("      <x>{}</x>", chosen_headings);
                } else {
                    section_output += &format!("{}\n", chosen_headings);
                }

                if chosen_headings.len() == 0 {
                    eprintln!("** have headings but no stuff chosen **")
                }
            }

            if templates.len() > 0 {
                let chosen_templates = "\n".to_owned() + &templates
                    .iter()
                    .map(|h| format!("        {}: {}", h.0, h.1))
                    .collect::<Vec<String>>()
                    .join("\n");

                if output_xml {
                    section_output += "\n";
                    section_output += &format!("      <t>{}</t>", chosen_templates);
                } else {
                    section_output += &format!("{}\n", chosen_templates);
                }
            }

            if output_xml {
                section_output += "\n";
                section_output += "    </s>";
            }

            sections_output_vec.push(section_output);
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
            *just_emitted_update = *page_num % 256 == 0;
            if *just_emitted_update {
                emit_update(output_xml, headings_seen);
            }
        }
    }
}

fn categorize_and_count(seen: &mut HeadingsSeen, headings: Vec<(String, u8)>) -> Vec<(String, u8)> {
    let mut nonblack_headings: Vec<(String, u8)> = Vec::new();

    for heading in headings {
        if HEADING_BLACKLIST.contains(&heading.0.as_str()) {
            *seen.black.entry(heading.0.clone()).or_insert(0) += 1;
        } else {
            if HEADING_WHITELIST.contains(&heading.0.as_str()) {
                *seen.white.entry(heading.0.clone()).or_insert(0) += 1;
            } else {
                *seen.grey.entry(heading.0.clone()).or_insert(0) += 1;
            }
            nonblack_headings.push(heading);
        }
    }
    nonblack_headings
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

// collect all headings and their depths from the text of a language section
fn get_headings_and_templates(langsect: &str) -> (Vec<(String, u8)>, Vec<(String, u8)>) {
    let all_headings_regex = Regex::new(r"(?m)^(===+) ?([^=]*?) ?===+ *$\n").unwrap();
    let mut headings: Vec<(String, u8)> = Vec::new();

    for capture in all_headings_regex.captures_iter(langsect) {
        let heading_depth = capture.get(1).unwrap().as_str().len();
        // let heading_name = capture.get(2).unwrap();
        // let name_string = heading_name.as_str().to_string();
        let name_string = capture.get(2).unwrap().as_str().to_string();

        headings.push((name_string, heading_depth.try_into().unwrap()));
    }

    let all_templates_regex = Regex::new(r"(?m)()\{\{(.*?)(:?\||}})").unwrap();
    let mut templates: Vec<(String, u8)> = Vec::new();

    // make a map to count how many times each template is seen
    let mut seen_map: HashMap<String, u8> = HashMap::new();

    for capture in all_templates_regex.captures_iter(langsect) {
        let template_name = capture.get(2).unwrap().as_str().to_string();

        // bump count of this template
        let seen_count = seen_map.entry(template_name.clone()).or_insert(0);
        *seen_count += 1;
        // templates.push((template_name, *seen_count));
    }

    for (template_name, count) in seen_map {
        templates.push((template_name, count));
    }

    templates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    
    (headings, templates)
}

fn emit_update(output_xml: bool, headings_seen: &mut HeadingsSeen) {
    match output_xml {
        true => println!("  <update>"),
        false => println!("--update--"),
    }

    for (headings, heading_name) in &[
        (&headings_seen.white, "white"),
        (&headings_seen.grey, "grey"),
        (&headings_seen.black, "black"),
    ] {
        if headings.len() == 0 {
            continue;
        }

        let mut headings: Vec<_> = headings.iter().collect();
        headings.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        match output_xml {
            true => println!("    <{}>", heading_name),
            false => println!("  {}", heading_name),
        }

        let fmt = match output_xml {
            true => |h: &str, c: &u64| format!("      <h n=\"{}\" c=\"{}\"/>", h, c),
            false => |h: &str, c: &u64| format!("    {}: {}", h, c),
        };

        println!("{}", headings.iter().map(|(h, c)| fmt(h, c)).collect::<Vec<String>>().join("\n"));

        match output_xml {
            true => println!("    </{}>", heading_name),
            false => println!(""),
        }
    }

    match output_xml {
        true => println!("  </update>"),
        false => println!(""),
    }
}
