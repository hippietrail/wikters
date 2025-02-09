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

fn start_page(
    page_title: &mut String,
    page_ns: &mut Option<i32>,
    page_id: &mut Option<i32>,
    page_rev_id: &mut Option<i32>,
    page_rev_text: &mut String,
) {
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

fn end_page(page_title: &String, page_ns: Option<i32>, page_id: Option<i32>, page_rev_id: Option<i32>, page_rev_text: &String) {
    // println!("page page/rev id {}/{} -> {} : \"{}\"", page_id.unwrap(), page_rev_id.unwrap(), page_ns.unwrap(), page_title);
    if page_ns.unwrap() == 0 {
        // Regex to match level-2 headings
        let heading_regex = Regex::new(r"^== ?([^=]*?) ?==$").unwrap();
        let mut languages: Vec<String> = Vec::new();

        for line in page_rev_text.lines() {
            if let Some(captures) = heading_regex.captures(line) {
                if let Some(heading) = captures.get(1) {
                    languages.push(heading.as_str().to_string());
                }
            }
        }

        // filter out all languages other than 'English' and 'Translingual'
        languages.retain(|lang| lang == "English" || lang == "Translingual");

        if languages.len() > 0 {
            println!("{} >>> Languages: {}", page_title, languages.join(", "));
        } else {
        }
    }
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

    loop {
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
                "page" => end_page(&page_title, page_ns, page_id, page_rev_id, &page_rev_text),
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
            Ok(Event::Eof) => break println!("Completed."),
            Ok(_) => {}
            Err(error) => break println!("{}", error),
        }

        // Clear the buffer for the next event
        buffer.clear();
    }

    Ok(())
}