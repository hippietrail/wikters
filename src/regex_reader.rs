use std::error::Error;
use std::io::BufRead;

use crate::{Page, PageSource};

pub struct RegexReader<R: BufRead> {
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

impl<R: BufRead> RegexReader<R> {
    pub fn new(reader: R) -> Self {
        RegexReader {
            lines: reader.lines(),
            state: State::PrePage,
            title: None,
            ns: None,
            pid: None,
            text_buffer: String::new(),
        }
    }
}

impl<R: BufRead> PageSource for RegexReader<R> {
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
                if let Some(title_start) = line.find("<title>") {
                    if let Some(title_end) = line[title_start..].find("</title>") {
                        let title_end = title_start + title_end;
                        self.title = Some(line[title_start + 7..title_end].to_string());
                    }
                }
                if let Some(ns_start) = line.find("<ns>") {
                    if let Some(ns_end) = line[ns_start..].find("</ns>") {
                        let ns_end = ns_start + ns_end;
                        self.ns = Some(line[ns_start + 4..ns_end].parse::<i32>()?);
                    }
                }
                if let Some(id_start) = line.find("<id>") {
                    if let Some(id_end) = line[id_start..].find("</id>") {
                        let id_end = id_start + id_end;
                        self.pid = Some(line[id_start + 4..id_end].parse::<i32>()?);
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
