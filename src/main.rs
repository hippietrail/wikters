use std::collections::HashMap;
use std::error::Error;
use std::io::{self, StdinLock};

use clap::Parser;
use quick_xml::{events::{BytesStart, Event}, name::QName, reader::Reader};
use regex::Regex;

mod heading_and_template_lists;
use heading_and_template_lists::{HEADING_BLACKLIST, HEADING_WHITELIST};
use heading_and_template_lists::{TEMPLATE_BLACKLIST, TEMPLATE_WHITELIST};

// Type aliases for complex tuple types
type Heading = (String, u8);
type Template = (String, u16);
type HeadingVec = Vec<Heading>;
type TemplateVec = Vec<Template>;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Limit the number of pages output.
    #[clap(short, long)]
    limit: Option<u64>,

    /// Output in lightweight XML format.
    #[clap(short, long)]
    xml: bool,

    /// No updates.
    #[clap(short, long)]
    no_updates: bool,

    /// Sample rate. Randomly pick an entry to include with a 1/n chance.
    #[clap(short, long)]
    sample_rate: Option<u64>,
}

struct Page {
    title: String,
    ns: Option<i32>,
    id: Option<i32>,
    rev_id: Option<i32>,
    rev_contrib_id: Option<i32>,
    rev_text: String,
}

impl Page {
    fn new() -> Self {
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

struct State {
    last_text_content: Option<String>,
    ns_key: Option<i32>,
    page: Page,

    page_num: u64,
    section_num: u64,

    headings_seen: Seen,
    templates_seen: Seen,

    just_emitted_update: bool,
}

struct Seen {
    white: HashMap<String, u64>, // headings I specifically want
    grey: HashMap<String, u64>,  // headings I didn't consider, rare headings, mistakes, etc.
    black: HashMap<String, u64>, // headings I specifically don't want
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

    let mut state = State {
        last_text_content: None,
        ns_key: None,
        page: Page::new(),
        page_num: 0,
        section_num: 0,
        headings_seen: Seen::new(),
        templates_seen: Seen::new(),
        just_emitted_update: false,
    };

    if args.xml {
        println!("<wiktionary>");
    }

    let mut qx_reader = Reader::from_reader(stdin.lock());
    let mut qx_buffer = Vec::new();

    while args.limit.is_none_or(|limit| state.page_num < limit) {
        if !qx_iterate(&args, &mut qx_reader, &mut qx_buffer, &mut state) {
            break;
        }
    }

    if !state.just_emitted_update && !args.no_updates {
        emit_update(args.xml, &mut state.headings_seen, &mut state.templates_seen);
    }

    if args.xml {
        println!("</wiktionary>");
    }

    Ok(())
}

// Called with nothing quick-xml specific when each </page> closing tag has been read

fn end_page(
    output_xml: bool,               // output format
    no_updates: bool,               // suppress updates
    page: &Page,                    // page's data
    page_num: &mut u64,             // count of chosen pages
    section_num: &mut u64,          // count of chosen sections (each page may have English, Translingual, or both)
    just_emitted_update: &mut bool, // flag so we don't emit the final update if we just emitted one
    headings_seen: &mut Seen,       // we count how many times we see each heading
    templates_seen: &mut Seen,      // we count how many times we see each template
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

    let mut page_output = match output_xml {
        true => format!(
            "  <p n=\"{}\" pid=\"{}\" rid=\"{}\">\n    <t>{}</t>",
            page_num,
            page.id.unwrap(),
            page.rev_id.unwrap(),
            page.title
        ),
        false => page.title.clone(),
    };

    // now split the text by the same regex
    let split_pagetext = our_lang_headings_regex.split(&page.rev_text).collect::<Vec<&str>>();

    let mut sections_output_vec: Vec<String> = Vec::new();

    // skip the prologue before the first heading, usually contains {{also}}
    for (i, langsectext) in split_pagetext.iter().enumerate().skip(1) {
        *section_num += 1;

        let mut section_output = match output_xml {
            true => format!("    <s n=\"{}\" l=\"{}\">", section_num, languages[i - 1]),
            false => format!("  {}", languages[i - 1]),
        };

        // get everything after this heading
        let mut langsectext = *langsectext;
        // but keep only up to the next heading
        if let Some(heading) = all_lang_headings_regex.find(langsectext) {
            langsectext = &langsectext[0..heading.start()];
        }

        let (headings, templates) = get_headings_and_templates(langsectext);
        let (nonblack_headings, white_templates) =
            categorize_and_count(headings_seen, headings, templates_seen, templates);

        if !nonblack_headings.is_empty() {
            let depth = output_xml as i32 * 4 - 2;

            let chosen_headings = "\n".to_owned()
                + &nonblack_headings
                    .iter()
                    .map(|h| format!("{:width$}{}", "", h.0, width = (h.1 as i32 * 2 + depth) as usize))
                    .collect::<Vec<String>>()
                    .join("\n");

            if output_xml {
                section_output += "\n";
                section_output += &format!("      <x>{}</x>", chosen_headings);
            } else {
                section_output += &format!("{}\n", chosen_headings);
            }

            if chosen_headings.is_empty() {
                eprintln!("** have headings but no stuff chosen **")
            }
        }

        if !white_templates.is_empty() {
            let chosen_templates = "\n".to_owned()
                + &white_templates
                    .iter()
                    .map(|h| format!("{:d$}{}: {}", "", h.0, h.1, d = if output_xml { 8 } else { 2 }))
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

    if !sections_output_vec.is_empty() {
        page_output += "\n";
        page_output += &sections_output_vec.join("\n");
        if output_xml {
            page_output += "\n";
            page_output += "  </p>";
        }
        println!("{}", page_output);

        // every n pages, emit an update
        *just_emitted_update = *page_num % 256 == 0;
        if *just_emitted_update && !no_updates {
            emit_update(output_xml, headings_seen, templates_seen);
        }
    }
}

fn categorize_and_count(
    seen_headings: &mut Seen,
    headings: HeadingVec,
    seen_templates: &mut Seen,
    templates: TemplateVec,
) -> (HeadingVec, TemplateVec) {
    let mut nonblack_headings: HeadingVec = Vec::new();

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

    let mut white_templates: TemplateVec = Vec::new();

    for (template, count) in templates {
        if TEMPLATE_BLACKLIST.contains(&template.as_str()) {
            *seen_templates.black.entry(template.clone()).or_insert(0) += count as u64;
        } else if TEMPLATE_WHITELIST.contains(&template.as_str()) {
            *seen_templates.white.entry(template.clone()).or_insert(0) += count as u64;
            white_templates.push((template.clone(), count));
        } else {
            *seen_templates.grey.entry(template.clone()).or_insert(0) += count as u64;
        }
    }

    (nonblack_headings, white_templates)
}

// from the text of a language section, collect all headings and their depths
// and all templates and their counts
fn get_headings_and_templates(langsect: &str) -> (HeadingVec, TemplateVec) {
    let all_headings_regex = Regex::new(r"(?m)^(===+) ?([^=]*?) ?===+ *$\n").unwrap();
    let mut headings: HeadingVec = Vec::new();

    for cap in all_headings_regex.captures_iter(langsect) {
        let heading_depth = cap.get(1).unwrap().as_str().len();
        let name_string = cap.get(2).unwrap().as_str().to_string();

        headings.push((name_string, heading_depth.try_into().unwrap()));
    }

    let all_templates_regex = Regex::new(r"(?m)\{\{([^|}:&]*[|:&])").unwrap();
    let mut templates: TemplateVec = Vec::new();

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

    if !headings_seen.white.is_empty() || !headings_seen.grey.is_empty() || !headings_seen.black.is_empty() {
        match output_xml {
            true => println!("    <headings>"),
            false => println!("  headings"),
        }
    }

    for (index, headings) in [&headings_seen.white, &headings_seen.grey, &headings_seen.black]
        .iter()
        .enumerate()
    {
        if headings.is_empty() {
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

        println!(
            "{}",
            headings
                .iter()
                .map(|(h, c)| fmt(h, c))
                .collect::<Vec<String>>()
                .join("\n")
        );

        match output_xml {
            true => println!("      </{}>", heading_name),
            false => println!(),
        }
    }

    if !headings_seen.white.is_empty() || !headings_seen.grey.is_empty() || !headings_seen.black.is_empty() {
        match output_xml {
            true => println!("    </headings>"),
            false => println!(),
        }
    }
    if !templates_seen.white.is_empty() || !templates_seen.grey.is_empty() || !templates_seen.black.is_empty() {
        match output_xml {
            true => println!("    <templates>"),
            false => println!("  templates"),
        }
    }

    for (index, templates) in [&templates_seen.white, &templates_seen.grey, &templates_seen.black]
        .iter()
        .enumerate()
    {
        if templates.is_empty() {
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

        println!(
            "{}",
            templates
                .iter()
                .map(|(h, c)| fmt(h, c))
                .collect::<Vec<String>>()
                .join("\n")
        );

        match output_xml {
            true => println!("      </{}>", template_name),
            false => println!(),
        }
    }

    if !templates_seen.white.is_empty() || !templates_seen.grey.is_empty() || !templates_seen.black.is_empty() {
        match output_xml {
            true => println!("    </templates>"),
            false => println!(),
        }
    }

    match output_xml {
        true => println!("  </update>"),
        false => println!(),
    }
}

/////////////// quick-xml stuff ///////////

// Does one 'iteration' of the quick-xml loop.
// This does not mean get the next page.
// In the quick-xml case it means one 'Event'
// Calls `end_page` when it gets to the </page> - calls with nothing quick-xml specific!

fn qx_iterate(
    args: &Args,
    qx_reader: &mut Reader<StdinLock<'static>>,
    qx_buffer: &mut Vec<u8>,
    state: &mut State,
) -> bool {
    match qx_reader.read_event_into(qx_buffer) {
        Ok(Event::Start(node)) => match node.name().as_ref() {
            b"namespace" => start_namespace(&node, &mut state.ns_key, &mut state.last_text_content),
            b"page" => start_page(&mut state.page),
            b"title" => start_page_title(&mut state.last_text_content),
            b"ns" => start_page_ns(&mut state.last_text_content, &mut state.page.ns),
            b"id" => start_id(&mut state.last_text_content),
            b"text" => start_page_rev_text(&mut state.last_text_content),
            _ => {}
        },
        Ok(Event::Empty(node)) => {
            if node.name().as_ref() == b"namespace" {
                start_namespace(&node, &mut state.ns_key, &mut state.last_text_content);
                end_namespace(state.ns_key, &state.last_text_content);
            }
        }
        Ok(Event::End(node)) => match node.name().as_ref() {
            b"namespace" => end_namespace(state.ns_key, &state.last_text_content),
            b"title" => end_page_title(&mut state.page.title, &mut state.last_text_content),
            b"ns" => end_page_ns(&mut state.page.ns, &mut state.last_text_content),
            b"id" => end_id(
                &mut state.page,
                &mut state.last_text_content,
            ),
            b"text" => end_page_rev_text(&mut state.page.rev_text, &mut state.last_text_content),
            b"page" => end_page(
                args.xml,
                args.no_updates,
                &state.page,
                &mut state.page_num,
                &mut state.section_num,
                &mut state.just_emitted_update,
                &mut state.headings_seen,
                &mut state.templates_seen,
            ),
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
fn start_namespace(node: &BytesStart, ns_key: &mut Option<i32>, last_text_content: &mut Option<String>) {
    if let Some(att) = node
        .attributes()
        .find(|a| a.as_ref().unwrap().key == QName(b"key"))
    {
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
fn end_namespace(_ns_key: Option<i32>, last_text_content: &Option<String>) {
    // The default namespace, 0, has no name
    let _ns_text = last_text_content.as_ref().unwrap_or(&String::from("")).clone();
    // println!("namespace {} : \"{}\"", ns_key.unwrap(), ns_text);
}

fn start_page(page: &mut Page) {
    *page = Page::new();
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

fn end_id(page: &mut Page, last_text_content: &mut Option<String>) {
    let id = last_text_content.take().unwrap_or_default().parse::<i32>().unwrap();
    if page.id.is_none() {
        page.id = Some(id);
    } else if page.rev_id.is_none() {
        page.rev_id = Some(id);
    } else if page.rev_contrib_id.is_none() {
        page.rev_contrib_id = Some(id);
    }
}

fn start_page_rev_text(last_text_content: &mut Option<String>) {
    *last_text_content = None;
}

fn end_page_rev_text(page_rev_text: &mut String, last_text_content: &mut Option<String>) {
    *page_rev_text = last_text_content.take().unwrap_or_default();
}
