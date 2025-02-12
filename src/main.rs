use std::collections::HashMap;
use std::error::Error;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io;
use regex::Regex;

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

fn end_namespace(ns_key: Option<i32>, last_text_content: &Option<String>) {
    // The default namespace, 0, has no name
    let ns_text = last_text_content.as_ref().unwrap_or(&String::from("")).clone();
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

fn end_page(title: &String, namespace: Option<i32>, page_id: Option<i32>, rev_id: Option<i32>, text: &String,
        page_num: &mut u64, section_num: &mut u64, just_emitted_update: &mut bool, headings_seen: &mut HashMap<String, u64>) {
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

        let mut page_xml = format!("  <p n=\"{}\" pid=\"{}\" rid=\"{}\">\n    <t>{}</t>",
            page_num, page_id.unwrap(), rev_id.unwrap(), title);

        // now split the text by the same regex
        let splitted = our_lang_headings_regex.split(text).collect::<Vec<&str>>();

        let mut sections_xml_vec: Vec<String> = Vec::new();

        for (i, section) in splitted.iter().enumerate().skip(1) {
            *section_num += 1;

            let mut section_xml = format!("    <s num=\"{}\" lang=\"{}\">", section_num, languages[i-1]);
            let mut inner_section = *section;

            if let Some(heading) = all_lang_headings_regex.find(section) {
                inner_section = &section[0..heading.start()];
            }
            // let chosen_stuff = inner_section;

            let all_headings_regex = Regex::new(r"(?m)^(===+) ?([^=]*?) ?===+ *$\n").unwrap();
            let mut headings: Vec<(String, u8)> = Vec::new();

            for capture in all_headings_regex.captures_iter(inner_section) {
                let heading_depth = capture.get(1).unwrap().as_str().len();
                let heading_name = capture.get(2).unwrap();
                let name_string = heading_name.as_str().to_string();

                headings.push((name_string, heading_depth.try_into().unwrap()));
            }

            let heading_blacklist = [
                "Anagrams",
                "Antonyms",
                "Collocations",
                "Conjugation",
                "Coordinate terms",
                "Derived characters",
                "Derived terms",
                "Descendants",
                // "Etymology", // keep because there can be multiple
                "Further reading",
                "Gallery",
                "Han character",
                "Holonyms",
                "Hypernyms",
                "Hyponyms",
                "Letter",
                "Meronyms",
                "Number",
                "Phrase",
                "Pronunciation",
                "Quotations",
                "References",
                "Related characters",
                "Related terms",
                "See also",
                "Statistics",
                "Symbol",
                "Synonyms",
                "Translations",
                "Trivia",
                "Unrelated terms",
                "Usage notes",
            ];

            headings.retain(|heading| !heading_blacklist.contains(&heading.0.as_str()));

            if headings.len() > 0 {
                let chosen_stuff = "\n".to_owned() + &headings
                    .iter()
                    .map(|heading| format!("{:width$}{}", "", heading.0, width = heading.1 as usize * 2 + 2))
                    .collect::<Vec<String>>()
                    .join("\n");

                // update seen heading count
                for heading in &headings {
                    *headings_seen.entry(heading.0.clone()).or_insert(0) += 1;
                }

                section_xml += "\n";
                section_xml += &format!("      <x>{}{}</x>",
                    "", //lang_headings[i-1],
                    chosen_stuff);
                section_xml += "\n";
                section_xml += "    </s>";

                if chosen_stuff.len() == 0 {
                    eprintln!("** have headings but no stuff chosen **")
                }

                sections_xml_vec.push(section_xml);
            }
        }

        if sections_xml_vec.len() > 0 {
            page_xml += "\n";
            page_xml += &sections_xml_vec.join("\n");

            page_xml += "\n";
            page_xml += "  </p>";
            println!("{}", page_xml);

            // every n pages, emit an update
            if *page_num %256 == 0 {
                emit_update(&headings_seen);
                *just_emitted_update = true;
            } else {
                *just_emitted_update = false;
            }
        }
    }
}

fn emit_update(headings_seen: &HashMap<String, u64>) {
    println!("  <update>");
    let mut sorted_headings: Vec<_> = headings_seen.iter().collect();
    sorted_headings.sort_by(|a, b| b.1.cmp(a.1));
    
    for (heading, count) in sorted_headings {
        println!("    <h n=\"{}\" c=\"{}\"/>", heading, count);
    }
    println!("  </update>");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
    let mut headings_seen: HashMap<String, u64> = HashMap::new();

    println!("<wiktionary>");

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
                "page" => end_page(&page_title, page_ns, page_id, page_rev_id, &page_rev_text,
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
            Ok(Event::Eof) => {}
            Ok(_) => {}
            Err(error) => break //println!("{}", error),
        }

        // Clear the buffer for the next event
        buffer.clear();
    }

    if !just_emitted_update {
        emit_update(&headings_seen);
    }

    println!("</wiktionary>");

    Ok(())
}