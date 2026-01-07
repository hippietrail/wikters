use std::error::Error;
use std::io::StdinLock;

use quick_xml::{
    events::{BytesStart, Event},
    name::QName,
    reader::Reader,
};

use crate::{Page, PageSource};

pub struct QuickXmlReader {
    reader: Reader<StdinLock<'static>>,
    buffer: Vec<u8>,
    last_text_content: Option<String>,
    ns_key: Option<i32>,
    page: Page,
}

impl QuickXmlReader {
    pub fn new(stdin: StdinLock<'static>) -> Self {
        QuickXmlReader {
            reader: Reader::from_reader(stdin),
            buffer: Vec::new(),
            last_text_content: None,
            ns_key: None,
            page: Page::new(),
        }
    }
}

impl PageSource for QuickXmlReader {
    fn next_page(&mut self) -> Result<Option<Page>, Box<dyn Error>> {
        loop {
            match self.reader.read_event_into(&mut self.buffer) {
                Ok(Event::Start(node)) => match node.name().as_ref() {
                    b"namespace" => start_namespace(&node, &mut self.ns_key, &mut self.last_text_content),
                    b"page" => self.page = Page::new(),
                    b"title" => self.last_text_content = None,
                    b"ns" => {
                        self.page.ns = None;
                        self.last_text_content = None;
                    }
                    b"id" => self.last_text_content = None,
                    b"text" => self.last_text_content = None,
                    _ => {}
                },
                Ok(Event::Empty(node)) => {
                    if node.name().as_ref() == b"namespace" {
                        start_namespace(&node, &mut self.ns_key, &mut self.last_text_content);
                        end_namespace(self.ns_key, &self.last_text_content);
                    }
                }
                Ok(Event::End(node)) => match node.name().as_ref() {
                    b"namespace" => end_namespace(self.ns_key, &self.last_text_content),
                    b"title" => {
                        self.page.title = self.last_text_content.take().unwrap_or_default();
                    }
                    b"ns" => {
                        let ns_text = self.last_text_content.take().unwrap_or_default();
                        self.page.ns = ns_text.parse::<i32>().ok();
                    }
                    b"id" => {
                        let id_str = self.last_text_content.take().unwrap_or_default();
                        let id = id_str.parse::<i32>()?;
                        if self.page.id.is_none() {
                            self.page.id = Some(id);
                        } else if self.page.rev_id.is_none() {
                            self.page.rev_id = Some(id);
                        } else if self.page.rev_contrib_id.is_none() {
                            self.page.rev_contrib_id = Some(id);
                        }
                    }
                    b"text" => {
                        self.page.rev_text = self.last_text_content.take().unwrap_or_default();
                    }
                    b"page" => {
                        let page = std::mem::replace(&mut self.page, Page::new());
                        self.buffer.clear();
                        return Ok(Some(page));
                    }
                    _ => {}
                },
                Ok(Event::Text(text)) => {
                    let s = String::from_utf8(text.to_vec())?;
                    if let Some(ref mut last_text_content) = self.last_text_content {
                        last_text_content.push_str(&s);
                    } else {
                        self.last_text_content = Some(s);
                    }
                }
                Ok(Event::Eof) => {
                    return Ok(None);
                }
                Ok(_) => {}
                Err(e) => {
                    return Err(Box::new(e));
                }
            }
            self.buffer.clear();
        }
    }
}

fn start_namespace(node: &BytesStart, ns_key: &mut Option<i32>, last_text_content: &mut Option<String>) {
    if let Some(att) = node.attributes().find(|a| a.as_ref().unwrap().key == QName(b"key")) {
        *ns_key = Some(
            String::from_utf8(att.unwrap().value.to_vec())
                .unwrap()
                .parse::<i32>()
                .unwrap(),
        );
    }
    *last_text_content = None;
}

fn end_namespace(_ns_key: Option<i32>, _last_text_content: &Option<String>) {
    // The default namespace, 0, has no name
}
