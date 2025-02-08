use std::error::Error;
use std::io::BufReader;
use std::fs::File;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use std::io;

fn getnodename(node: &quick_xml::events::BytesStart) -> String {
    String::from_utf8(node.name().0.to_vec()).unwrap()
}
fn getnodenameend(node: &quick_xml::events::BytesEnd) -> String {
    String::from_utf8(node.name().0.to_vec()).unwrap()
}

fn handle_namespace(node: &quick_xml::events::BytesStart, current_ns_key: &mut Option<String>, current_text_content: &mut Option<String>) {
    // Capture the key from the attributes
    for att in node.attributes() {
        let att = att.unwrap();
        if att.key == quick_xml::name::QName(b"key") {
            *current_ns_key = Some(String::from_utf8(att.value.to_vec()).unwrap());
        }
    }
    // Reset text content for the namespace
    *current_text_content = None; // Reset for each namespace
}

fn print_namespace(current_ns_key: &mut Option<String>, current_text_content: &mut Option<String>) {
    if let Some(k) = current_ns_key.take() {
        let ns_text = current_text_content.take().unwrap_or_else(|| String::from("")); // Default to empty string if None
        println!("namespace {} : \"{}\"", k, ns_text); // Print key and text content (or empty)
    }
}

fn handle_page(node: &quick_xml::events::BytesStart, current_page_title: &mut Option<String>, current_text_content: &mut Option<String>) {
    for att in node.attributes() {
        let att = att.unwrap();
        if att.key == quick_xml::name::QName(b"title") {
            *current_page_title = Some(String::from_utf8(att.value.to_vec()).unwrap());
        }
    }
    *current_text_content = None;
}

fn print_page(current_text_content: &mut Option<String>) {
    let ns_text = current_text_content.take().unwrap_or_else(|| String::from("")); // Default to empty string if None
    println!("page \"{}\"", ns_text);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the reader to read from standard input
    let stdin = io::stdin();
    let mut reader = Reader::from_reader(stdin.lock());
    // let file = File::open("fake.xml")?;
    // let br = BufReader::new(file);
    // let mut reader = Reader::from_reader(br);

    // Initialize variables to track the path and seen attributes
    let mut current_text_content: Option<String> = None;
    let mut current_ns_key: Option<String> = None;
    let mut current_page_title: Option<String> = None;

    // Buffer to hold the current event data
    let mut buffer = Vec::new();

    // Main loop to read events from the XML input
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(node)) => {
                if getnodename(&node) == "namespace" {
                    handle_namespace(&node, &mut current_ns_key, &mut current_text_content);
                } else if getnodename(&node) == "title" {
                    handle_page(&node, &mut current_page_title, &mut current_text_content);
                }
            },
            Ok(Event::Empty(node)) => {
                if getnodename(&node) == "namespace" {
                    handle_namespace(&node, &mut current_ns_key, &mut current_text_content);
                    print_namespace(&mut current_ns_key, &mut current_text_content); // Print for empty namespace
                }
            },
            Ok(Event::End(node)) => {
                if getnodenameend(&node) == "namespace" {
                    print_namespace(&mut current_ns_key, &mut current_text_content); // Print for end of namespace
                } else if getnodenameend(&node) == "title" {
                    print_page(&mut current_text_content); // Print for end of page
                }
            }
            Ok(Event::Text(text)) => {
                // Capture text content for the namespace
                if let Some(ref mut t) = current_text_content {
                    t.push_str(&String::from_utf8(text.to_vec()).unwrap());
                } else {
                    current_text_content = Some(String::from_utf8(text.to_vec()).unwrap());
                }
            }
            Ok(Event::Eof) => break println!("Completed."),
            Ok(_) => {
            }
            Err(error) => break println!("{}", error),
        }

        // Clear the buffer for the next event
        buffer.clear();
    }

    Ok(())
}

/*

<mediawiki xmlns="http://www.mediawiki.org/xml/export-0.11/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.mediawiki.org/xml/export-0.11/ http://www.mediawiki.org/xml/export-0.11.xsd" version="0.11" xml:lang="en">
  <siteinfo>
    <sitename>Wiktionary</sitename>
    <dbname>enwiktionary</dbname>
    <base>https://en.wiktionary.org/wiki/Wiktionary:Main_Page</base>
    <generator>MediaWiki 1.44.0-wmf.14</generator>
    <case>case-sensitive</case>
    <namespaces>
      <namespace key="-2" case="case-sensitive">Media</namespace>
      <namespace key="-1" case="first-letter">Special</namespace>
      <namespace key="0" case="case-sensitive" />
      <namespace key="1" case="case-sensitive">Talk</namespace>
      <namespace key="2" case="first-letter">User</namespace>
      <namespace key="3" case="first-letter">User talk</namespace>
      <namespace key="4" case="case-sensitive">Wiktionary</namespace>
      <namespace key="5" case="case-sensitive">Wiktionary talk</namespace>
      <namespace key="6" case="case-sensitive">File</namespace>
      <namespace key="7" case="case-sensitive">File talk</namespace>
      <namespace key="8" case="first-letter">MediaWiki</namespace>
      <namespace key="9" case="first-letter">MediaWiki talk</namespace>
      <namespace key="10" case="case-sensitive">Template</namespace>
      <namespace key="11" case="case-sensitive">Template talk</namespace>
      <namespace key="12" case="case-sensitive">Help</namespace>
      <namespace key="13" case="case-sensitive">Help talk</namespace>
      <namespace key="14" case="case-sensitive">Category</namespace>
      <namespace key="15" case="case-sensitive">Category talk</namespace>
      <namespace key="90" case="case-sensitive">Thread</namespace>
      <namespace key="91" case="case-sensitive">Thread talk</namespace>
      <namespace key="92" case="case-sensitive">Summary</namespace>
      <namespace key="93" case="case-sensitive">Summary talk</namespace>
      <namespace key="100" case="case-sensitive">Appendix</namespace>
      <namespace key="101" case="case-sensitive">Appendix talk</namespace>
      <namespace key="106" case="case-sensitive">Rhymes</namespace>
      <namespace key="107" case="case-sensitive">Rhymes talk</namespace>
      <namespace key="108" case="case-sensitive">Transwiki</namespace>
      <namespace key="109" case="case-sensitive">Transwiki talk</namespace>
      <namespace key="110" case="case-sensitive">Thesaurus</namespace>
      <namespace key="111" case="case-sensitive">Thesaurus talk</namespace>
      <namespace key="114" case="case-sensitive">Citations</namespace>
      <namespace key="115" case="case-sensitive">Citations talk</namespace>
      <namespace key="116" case="case-sensitive">Sign gloss</namespace>
      <namespace key="117" case="case-sensitive">Sign gloss talk</namespace>
      <namespace key="118" case="case-sensitive">Reconstruction</namespace>
      <namespace key="119" case="case-sensitive">Reconstruction talk</namespace>
      <namespace key="710" case="case-sensitive">TimedText</namespace>
      <namespace key="711" case="case-sensitive">TimedText talk</namespace>
      <namespace key="828" case="case-sensitive">Module</namespace>
      <namespace key="829" case="case-sensitive">Module talk</namespace>
    </namespaces>
  </siteinfo>
  <page>
    <title>Wiktionary:Welcome, newcomers</title>
    <ns>4</ns>
    <id>6</id>
    <revision>
      <id>83502358</id>
      <parentid>80638725</parentid>
      <timestamp>2025-01-07T10:50:12Z</timestamp>
      <contributor>
        <username>Hftf</username>
        <id>1987641</id>
      </contributor>
      <minor />
      <comment>super duper annoying as a user to read this start-here page full of easter egg links half of which go to other relevant About pages as expected and half of which are arbitrarily linked dictionary entries. one sentence of easter eggs is enough</comment>
      <origin>83502358</origin>
      <model>wikitext</model>
      <format>text/x-wiki</format>
      <text bytes="6392" sha1="qjh11899zdlpywps07kfdb0d8i3g1l8" xml:space="preserve">{{shortcut|Project:About}}</text>
    </revision>
  </page>

*/