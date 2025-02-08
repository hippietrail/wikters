use std::collections::HashMap;
use std::error::Error;

use quick_xml::events::{Event, BytesStart};
use quick_xml::reader::Reader;

use std::io::{self, BufRead, Write};

fn print_namespaces(node: BytesStart) {
    let mut key: Option<String> = None;
    let mut text: Option<String> = None;

    // Extract the key from the attributes
    for attribute in node.attributes() {
        let attribute = attribute.unwrap();
        if attribute.key == quick_xml::name::QName(b"key") {
            key = Some(String::from_utf8(attribute.value.to_vec()).unwrap());
        }
    }

    // Extract the text content of the namespace element
    text = Some(String::from_utf8(node.name().0.to_vec()).unwrap());

    // Print the key and text content together
    if let (Some(k), Some(t)) = (key, text) {
        println!("{} : {}", k, t);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the reader to read from standard input
    let stdin = io::stdin();
    let mut reader = Reader::from_reader(stdin.lock());

    // Initialize variables to track the path and seen attributes
    let mut path = Vec::new();
    let mut current_key: Option<String> = None;
    let mut current_text: Option<String> = None;

    // Buffer to hold the current event data
    let mut buffer = Vec::new();
    let mut event_count = 0;

    // Main loop to read events from the XML input
    loop {
        match reader.read_event_into(&mut buffer) {
            Err(error) => break println!("{}", error),
            Ok(Event::Eof) => break println!("Completed."),
            Ok(Event::Start(node)) => {
                let node_name = String::from_utf8(node.name().0.to_vec()).unwrap();
                if node_name == "namespace" {
                    // Capture the key from the attributes
                    for attribute in node.attributes() {
                        let attribute = attribute.unwrap();
                        if attribute.key == quick_xml::name::QName(b"key") {
                            current_key = Some(String::from_utf8(attribute.value.to_vec()).unwrap());
                        }
                    }
                }
                path.push(format!("{:?}", node_name));
                event_count += 1;
                if event_count >= 100000 {
                    break println!("Reached 100000 events");
                }
            }
            Ok(Event::Text(text)) => {
                current_text = Some(String::from_utf8(text.to_vec()).unwrap());
            }
            Ok(Event::End(node)) => {
                let node_name = String::from_utf8(node.name().0.to_vec()).unwrap();
                if node_name == "namespace" {
                    if let (Some(k), Some(t)) = (current_key.take(), current_text.take()) {
                        println!("{} : {}", k, t);
                    }
                }
                path.pop();
            }
            Ok(_) => {
                // Handle other types of XML nodes
                // println!("Other element");
            }
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