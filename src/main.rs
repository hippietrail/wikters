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

fn handle_namespace(node: &quick_xml::events::BytesStart, current_ns_key: &mut i32, current_text_content: &mut Option<String>) {
    for att in node.attributes() {
        let att = att.unwrap();
        if att.key == quick_xml::name::QName(b"key") {
            *current_ns_key = String::from_utf8(att.value.to_vec()).unwrap().parse::<i32>().unwrap();
        }
    }
    *current_text_content = None; // Reset for each namespace
}

fn print_namespace(current_ns_key: &i32, current_text_content: &Option<String>) {
    let ns_text = current_text_content.as_ref().unwrap_or(&String::from("")).clone(); // Default to empty string if None
    println!("namespace {} : \"{}\"", current_ns_key, ns_text); // Print key and text content (or empty)
}

fn handle_page_title(current_text_content: &mut Option<String>) {
    *current_text_content = None; // Reset text content for the page title
}

fn end_page_title(current_page_title: &mut String, current_text_content: &mut Option<String>) {
    *current_page_title = current_text_content.take().unwrap_or_default(); // Default to empty string if None
}

fn start_page_ns(current_text_content: &mut Option<String>, current_page_ns: &mut i32) {
    *current_page_ns = -777; // Placeholder for actual logic
    *current_text_content = None; // Reset for each namespace
}

fn end_page_ns(current_page_ns: &mut i32, current_text_content: &mut Option<String>) {
//fn end_page_ns(current_text_content: &mut Option<String>, current_page_ns: &mut i32) {
    let ns_text = current_text_content.take().unwrap_or_default(); // Default to empty string if None
    *current_page_ns = ns_text.parse::<i32>().unwrap(); // Parse ns text to i32
}

fn end_page(current_page_title: &String, current_page_ns: i32) {
    println!("page {} \"{}\"", current_page_ns, current_page_title);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let mut reader = Reader::from_reader(stdin.lock());

    let mut current_text_content: Option<String> = None;
    let mut current_ns_key: i32 = -333;
    let mut current_page_title: String = String::new();
    let mut current_page_ns: i32 = -444;

    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(node)) => match get_node_name(&node).as_str() {
                "namespace" => handle_namespace(&node, &mut current_ns_key, &mut current_text_content),
                "title" => handle_page_title(&mut current_text_content),
                "ns" => start_page_ns(&mut current_text_content, &mut current_page_ns),
                _ => {}
            },
            Ok(Event::Empty(node)) => match get_node_name(&node).as_str() {
                "namespace" => {
                    handle_namespace(&node, &mut current_ns_key, &mut current_text_content);
                    print_namespace(&current_ns_key, &current_text_content);
                },
                _ => {}
            },
            Ok(Event::End(node)) => match get_node_name_end(&node).as_str() {
                "namespace" => print_namespace(&current_ns_key, &current_text_content),
                "title" => end_page_title(&mut current_page_title, &mut current_text_content),
                "ns" => end_page_ns(&mut current_page_ns, &mut current_text_content),
                "page" => end_page(&current_page_title, current_page_ns),
                _ => {}
            },
            Ok(Event::Text(text)) => {
                let s = String::from_utf8(text.to_vec()).unwrap();
                if let Some(ref mut current_text_content) = current_text_content {
                    current_text_content.push_str(&s);
                } else {
                    current_text_content = Some(s);
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