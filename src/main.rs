use std::collections::HashMap;
use std::error::Error;
use std::io;

use clap::Parser;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use regex::Regex;

mod heading_and_template_lists;
use heading_and_template_lists::{HEADING_WHITELIST, HEADING_BLACKLIST};
use heading_and_template_lists::{TEMPLATE_WHITELIST, TEMPLATE_BLACKLIST};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Limit the number of pages output.
    #[clap(short, long)]
    limit: Option<u64>,

    /// Output in lightweight XML format.
    #[clap(short, long)]
    xml: bool,
}

struct Seen {
    white: HashMap<String, u64>,    // headings I specifically want
    grey: HashMap<String, u64>,     // headings I didn't consider, rare headings, mistakes, etc.
    black: HashMap<String, u64>,    // headings I specifically don't want
}

impl Seen {
    fn new() -> Self {
        Seen {
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

    let mut headings_seen = Seen::new();
    let mut templates_seen = Seen::new();

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
                    &mut page_num, &mut section_num, &mut just_emitted_update, &mut headings_seen, &mut templates_seen),
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
        emit_update(args.xml, &mut headings_seen, &mut templates_seen);
    }

    if args.xml {
        println!("</wiktionary>");
    }

    Ok(())
}

fn end_page(
    output_xml: bool,               // output format
    title: &String,                 // page's title from the dump
    namespace: Option<i32>,         // page's namespace
    page_id: Option<i32>,           // page's id
    rev_id: Option<i32>,            // page's revision id (history dumps have many, our dumps have one)
    pagetext: &String,              // page text
    page_num: &mut u64,             // count of chosen pages
    section_num: &mut u64,          // count of chosen sections (each page may have English, Translingual, or both)
    just_emitted_update: &mut bool, // flag so we don't emit the final update if we just emitted one
    headings_seen: &mut Seen,       // we count how many times we see each heading
    templates_seen: &mut Seen,      // we count how many times we see each template
) {
    if namespace.unwrap() != 0 { return; }
    
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

    let mut page_output = match output_xml {
        true => format!("  <p n=\"{}\" pid=\"{}\" rid=\"{}\">\n    <t>{}</t>",
            page_num, page_id.unwrap(), rev_id.unwrap(), title),
        false => title.clone(),
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

        let (headings, templates) = get_headings_and_templates(langsectext);
        let (nonblack_headings, white_templates) = categorize_and_count(headings_seen, headings, templates_seen, templates);

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

        if white_templates.len() > 0 {
            let chosen_templates = "\n".to_owned() + &white_templates
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
            emit_update(output_xml, headings_seen, templates_seen);
        }
    }
}

fn categorize_and_count(
    seen_headings: &mut Seen, headings: Vec<(String, u8)>,
    seen_templates: &mut Seen, templates: Vec<(String, u16)>
) -> (Vec<(String, u8)>, Vec<(String, u16)>) {
    let mut nonblack_headings: Vec<(String, u8)> = Vec::new();

    for heading in headings {
        if HEADING_BLACKLIST.contains(&heading.0.as_str()) {
            *seen_headings.black.entry(heading.0.clone()).or_insert(0) += 1;
        } else {
            if HEADING_WHITELIST.contains(&heading.0.as_str()) {
                *seen_headings.white.entry(heading.0.clone()).or_insert(0) += 1;
            } else {
                *seen_headings.grey.entry(heading.0.clone()).or_insert(0) += 1;
            }
            nonblack_headings.push(heading);
        }
    }

    let mut white_templates: Vec<(String, u16)> = Vec::new();

    for (template, count) in templates {
        if TEMPLATE_BLACKLIST.contains(&template.as_str()) {
            *seen_templates.black.entry(template.clone()).or_insert(0) += count as u64;
        } else {
            if TEMPLATE_WHITELIST.contains(&template.as_str()) {
                *seen_templates.white.entry(template.clone()).or_insert(0) += count as u64;
                white_templates.push((template.clone(), count.into()));
            } else {
                *seen_templates.grey.entry(template.clone()).or_insert(0) += count as u64;
            }
        }
    }

    (nonblack_headings, white_templates)
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

// from the text of a language section, collect all headings and their depths
// and all templates and their counts
fn get_headings_and_templates(langsect: &str) -> (Vec<(String, u8)>, Vec<(String, u16)>) {
    let all_headings_regex = Regex::new(r"(?m)^(===+) ?([^=]*?) ?===+ *$\n").unwrap();
    let mut headings: Vec<(String, u8)> = Vec::new();

    for cap in all_headings_regex.captures_iter(langsect) {
        let heading_depth = cap.get(1).unwrap().as_str().len();
        let name_string = cap.get(2).unwrap().as_str().to_string();

        headings.push((name_string, heading_depth.try_into().unwrap()));
    }

    let all_templates_regex = Regex::new(r"(?m)\{\{([^|}:&]*[|:&])").unwrap();
    let mut templates: Vec<(String, u16)> = Vec::new();

    let mut seen_map: HashMap<String, u16> = HashMap::new();

    for cap in all_templates_regex.captures_iter(langsect) {
        let mut template_name = cap.get(1).unwrap().as_str().to_string();
        let lc = template_name.chars().last().unwrap();
        // starts with &lt; if the template contains an html comment
        if ['|', '&'].contains(&lc) {
            template_name.pop();
            template_name = template_name.trim_end().to_string();
        }
        let seen_count = seen_map.entry(template_name.clone()).or_insert(0);
        *seen_count += 1;
    }

    for (template_name, count) in seen_map {
        templates.push((template_name, count));
    }

    templates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    
    (headings, templates)
}

fn emit_update(output_xml: bool, headings_seen: &mut Seen, templates_seen: &mut Seen) {
    let colours = ["white", "grey", "black"];

    match output_xml {
        true => println!("  <update>"),
        false => println!("--update--"),
    }

    if headings_seen.white.len() != 0 || headings_seen.grey.len() != 0 || headings_seen.black.len() != 0 {
        match output_xml {
            true => println!("    <headings>"),
            false => println!("  headings"),
        }
    }

    for (index, headings) in [
        &headings_seen.white, &headings_seen.grey, &headings_seen.black
    ].iter().enumerate() {
        if headings.len() == 0 {
            continue;
        }

        let heading_name = &colours[index];

        let mut headings: Vec<_> = headings.iter().collect();
        headings.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        match output_xml {
            true => println!("      <{}>", heading_name),
            false => println!("    {}", heading_name),
        }

        let fmt = match output_xml {
            true => |h: &str, c: &u64| format!("        <h n=\"{}\" c=\"{}\"/>", h, c),
            false => |h: &str, c: &u64| format!("      {}: {}", h, c),
        };

        println!("{}", headings.iter().map(|(h, c)| fmt(h, c)).collect::<Vec<String>>().join("\n"));

        match output_xml {
            true => println!("      </{}>", heading_name),
            false => println!(""),
        }
    }

    if headings_seen.white.len() != 0 || headings_seen.grey.len() != 0 || headings_seen.black.len() != 0 {
        match output_xml {
            true => println!("    </headings>"),
            false => println!(""),
        }
    }
    if templates_seen.white.len() != 0 || templates_seen.grey.len() != 0 || templates_seen.black.len() != 0 {
        match output_xml {
            true => println!("    <templates>"),
            false => println!("  templates"),
        }
    }

    for (index, templates) in [
        &templates_seen.white, &templates_seen.grey, &templates_seen.black
    ].iter().enumerate() {
        if templates.len() == 0 {
            continue;
        }

        let template_name = &colours[index];

        let mut templates: Vec<_> = templates.iter().collect();
        templates.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        match output_xml {
            true => println!("      <{}>", template_name),
            false => println!("    {}", template_name),
        }

        let fmt = match output_xml {
            true => |h: &str, c: &u64| format!("        <t n=\"{}\" c=\"{}\"/>", h, c),
            false => |h: &str, c: &u64| format!("      {}: {}", h, c),
        };

        println!("{}", templates.iter().map(|(h, c)| fmt(h, c)).collect::<Vec<String>>().join("\n"));

        match output_xml {
            true => println!("      </{}>", template_name),
            false => println!(""),
        }
    }

    if templates_seen.white.len() != 0 || templates_seen.grey.len() != 0 || templates_seen.black.len() != 0 {
        match output_xml {
            true => println!("    </templates>"),
            false => println!(""),
        }
    }

    match output_xml {
        true => println!("  </update>"),
        false => println!(""),
    }
}
