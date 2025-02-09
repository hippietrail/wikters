use std::error::Error;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io;

fn get_node_name(node: &quick_xml::events::BytesStart) -> String {
    String::from_utf8(node.name().0.to_vec()).unwrap()
}

fn get_node_name_end(node: &quick_xml::events::BytesEnd) -> String {
    String::from_utf8(node.name().0.to_vec()).unwrap()
}

fn start_namespace(node: &quick_xml::events::BytesStart, last_ns_key: &mut i32, last_text_content: &mut Option<String>) {
    if let Some(att) = node.attributes().find(|a| a.as_ref().unwrap().key == quick_xml::name::QName(b"key")) {
        *last_ns_key = String::from_utf8(att.unwrap().value.to_vec()).unwrap().parse::<i32>().unwrap();
    }
    *last_text_content = None; // Reset for each namespace
}

fn end_namespace(last_ns_key: i32, last_text_content: &Option<String>) {
    // The default namespace, 0, has no name
    let ns_text = last_text_content.as_ref().unwrap_or(&String::from("")).clone();
    println!("namespace {} : \"{}\"", last_ns_key, ns_text);
}

fn start_page(last_page_title: &mut String, last_page_ns: &mut i32, last_page_id: &mut Option<i32>) {
    *last_page_title = String::new();
    *last_page_ns = -555;
    *last_page_id = None;
}

fn start_page_title(last_text_content: &mut Option<String>) {
    *last_text_content = None; // Reset text content for the page title
}

fn end_page_title(last_page_title: &mut String, last_text_content: &mut Option<String>) {
    *last_page_title = last_text_content.take().unwrap_or_default(); // Default to empty string if None
}

fn start_page_ns(last_text_content: &mut Option<String>, last_page_ns: &mut i32) {
    *last_page_ns = -777; // Placeholder for actual logic
    *last_text_content = None; // Reset for each namespace
}

fn end_page_ns(last_page_ns: &mut i32, last_text_content: &mut Option<String>) {
    let ns_text = last_text_content.take().unwrap_or_default(); // Default to empty string if None
    *last_page_ns = ns_text.parse::<i32>().unwrap(); // Parse ns text to i32
}

fn start_id(last_text_content: &mut Option<String>/*, last_id: &mut Option<i32>*/) {
    // *last_id = None; // Placeholder for actual logic
    *last_text_content = None; // Reset for each namespace
}

fn end_id(/*last_id: &mut Option<i32>, */page_id: &mut Option<i32>, page_rev_id: &mut Option<i32>, page_rev_contrib_id: &mut Option<i32>, last_text_content: &mut Option<String>) {
    let id = last_text_content.take().unwrap_or_default().parse::<i32>().unwrap();
    // *last_id = Some(id); // Parse ns text to i32
    // out of the page, rev, and contrib id, write this into the first of them that is None
    if page_id.is_none() {
        *page_id = Some(id);
    } else if page_rev_id.is_none() {
        *page_rev_id = Some(id);
    } else if page_rev_contrib_id.is_none() {
        *page_rev_contrib_id = Some(id);
    }
}

fn end_page(last_page_title: &String, last_page_ns: i32, last_page_id: Option<i32>) {
    println!("page id {} -> {} : \"{}\"", last_page_id.unwrap(), last_page_ns, last_page_title);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let mut reader = Reader::from_reader(stdin.lock());

    let mut last_text_content: Option<String> = None;
    let mut last_ns_key: i32 = -333;
    let mut last_page_title: String = String::new();
    let mut last_page_ns: i32 = -444;
    // let mut last_id: Option<i32> = None;
    let mut page_id: Option<i32> = None;
    let mut page_rev_id: Option<i32> = None;
    let mut page_rev_contrib_id: Option<i32> = None;

    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(node)) => match get_node_name(&node).as_str() {
                "namespace" => start_namespace(&node, &mut last_ns_key, &mut last_text_content),
                "page" => start_page(&mut last_page_title, &mut last_page_ns, &mut page_id),
                "title" => start_page_title(&mut last_text_content),
                "ns" => start_page_ns(&mut last_text_content, &mut last_page_ns),
                "id" => start_id(&mut last_text_content/*, &mut last_id*/),
                _ => {}
            },
            Ok(Event::Empty(node)) => match get_node_name(&node).as_str() {
                "namespace" => {
                    start_namespace(&node, &mut last_ns_key, &mut last_text_content);
                    end_namespace(last_ns_key, &last_text_content);
                },
                _ => {}
            },
            Ok(Event::End(node)) => match get_node_name_end(&node).as_str() {
                "namespace" => end_namespace(last_ns_key, &last_text_content),
                "title" => end_page_title(&mut last_page_title, &mut last_text_content),
                "ns" => end_page_ns(&mut last_page_ns, &mut last_text_content),
                "id" => end_id(/*&mut last_id, */&mut page_id, &mut page_rev_id, &mut page_rev_contrib_id, &mut last_text_content),
                "page" => end_page(&last_page_title, last_page_ns, page_id),
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