use std::error::Error;
use std::io::BufRead;

use crate::{Page, PageSource};

pub struct StringOpsReader<R: BufRead> {
    lines: std::io::Lines<R>,
    state: State,
    title: Option<String>,
    ns: Option<i32>,
    pid: Option<i32>,
    text_buffer: String,
}

#[derive(PartialEq)]
enum State {
    /// Initial state, waiting for a <page> tag
    PrePage,

    /// Inside <page> but haven't found <title> and <id> yet
    InPage,

    /// Inside <page> and have found both title and page ID
    InPageAfterTitleAndId,

    /// Inside a <revision> tag
    InRevision,

    /// Inside the text content
    InRevisionText,
}

impl<R: BufRead> StringOpsReader<R> {
    pub fn new(reader: R) -> Self {
        StringOpsReader {
            lines: reader.lines(),
            state: State::PrePage,
            title: None,
            ns: None,
            pid: None,
            text_buffer: String::new(),
        }
    }
}

impl<R: BufRead> PageSource for StringOpsReader<R> {
    fn next_page(&mut self) -> Result<Option<Page>, Box<dyn Error>> {
        loop {
            let line = match self.lines.next() {
                Some(Ok(l)) => l,
                Some(Err(e)) => return Err(Box::new(e)),
                None => return Ok(None),
            };

            if self.state == State::PrePage {
                if line.contains("<page>") {
                    self.state = State::InPage;
                }
            } else if self.state == State::InPage {
                // Extract title
                if let Some(start) = line.find("<title>") {
                    if let Some(end) = line[start + 7..].find("</title>") {
                        self.title = Some(line[start + 7..start + 7 + end].to_string());
                    }
                }
                
                // Extract namespace
                if let Some(start) = line.find("<ns>") {
                    if let Some(end) = line[start + 4..].find("</ns>") {
                        let ns_str = &line[start + 4..start + 4 + end];
                        self.ns = Some(ns_str.parse::<i32>()?);
                    }
                }
                
                // Extract page ID
                if let Some(start) = line.find("<id>") {
                    if let Some(end) = line[start + 4..].find("</id>") {
                        let id_str = &line[start + 4..start + 4 + end];
                        self.pid = Some(id_str.parse::<i32>()?);
                    }
                }
                
                if self.title.is_some() && self.pid.is_some() {
                    self.state = State::InPageAfterTitleAndId;
                }
            } else if self.state == State::InPageAfterTitleAndId {
                if line.contains("<revision>") {
                    self.state = State::InRevision;
                } else if line.contains("</page>") {
                    let page = Page {
                        title: self.title.take().unwrap_or_default(),
                        ns: self.ns,
                        id: self.pid,
                        rev_id: Some(-1),
                        rev_contrib_id: None,
                        rev_text: self.text_buffer.clone(),
                    };
                    self.pid = None;
                    self.ns = None;
                    self.text_buffer.clear();
                    self.state = State::PrePage;
                    return Ok(Some(page));
                }
            } else if self.state == State::InRevision {
                if line.contains("<text") {
                    self.state = State::InRevisionText;
                    if let Some(end_tag) = line.find(">") {
                        self.text_buffer.push_str(&line[end_tag + 1..]);
                        self.text_buffer.push_str("\n");
                    }
                } else if line.contains("</revision>") {
                    self.state = State::InPageAfterTitleAndId;
                }
            } else if self.state == State::InRevisionText {
                self.text_buffer.push_str(&line);
                if line.contains("</text>") {
                    self.state = State::InPageAfterTitleAndId;
                    self.text_buffer.truncate(self.text_buffer.len() - 7);
                } else {
                    self.text_buffer.push_str("\n");
                }
            }
        }
    }
}
