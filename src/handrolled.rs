use std::io::BufRead;
// use crate::Args;
use crate::parse_page_wikitext;
use crate::Page;
// use crate::Seen;

use crate::Opts;

// pub fn process_dump(args: &Args, reader: impl BufRead) -> Result<(), Box<dyn std::error::Error>> {
pub fn process_dump(opts: &Opts, reader: impl BufRead) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(PartialEq)]
    enum State {
        /// Initial state, waiting for a <page> tag
        PrePage,

        /// Inside <page> but haven't found <title> and <id> yet
        /// This state is looking for both title and page ID
        InPage,

        /// Inside <page> and have found both title and page ID
        /// This state is looking for <revision> tags
        InPageAfterTitleAndId,

        /// Inside a <revision> tag
        /// This state is looking for the text content
        InRevision,

        /// Inside the text content
        /// This state accumulates text until </text> is found
        InRevisionText,
    }

    let mut pc = 0; // page count
    let mut rc = 0; // revision count for this page
    let mut _rc_all = 0; // revision count over all pages
    let mut title = None; // page title
    let mut ns = None; // namespace
    let mut pid = None; // page ID
    let mut state = State::PrePage;
    // let progress_modulo = if args.no_updates { 1000 } else { 10000 };
    let progress_modulo = if opts.no_updates { 1000 } else { 10000 };
    let mut show_progress = false;
    let mut text_buffer = String::new(); // Buffer to accumulate text content

    for line in reader.lines() {
        let line = line?;

        if state == State::PrePage {
            if line.contains("<page>") {
                if (pc % progress_modulo) == 0 {
                    show_progress = true;
                }
                state = State::InPage;
                rc = 0;
            }
        } else if state == State::InPage {
            if let Some(title_start) = line.find("<title>") {
                if let Some(title_end) = line[title_start..].find("</title>") {
                    let title_end = title_start + title_end;
                    title = Some(line[title_start + 7..title_end].to_string());
                }
            }
            if let Some(ns_start) = line.find("<ns>") {
                if let Some(ns_end) = line[ns_start..].find("</ns>") {
                    let ns_end = ns_start + ns_end;
                    ns = Some(line[ns_start + 4..ns_end].parse::<i32>()?);
                }
            }
            if let Some(id_start) = line.find("<id>") {
                if let Some(id_end) = line[id_start..].find("</id>") {
                    let id_end = id_start + id_end;
                    pid = Some(line[id_start + 4..id_end].parse::<i32>()?);
                }
            }
            if title.is_some() && pid.is_some() {
                state = State::InPageAfterTitleAndId;
            }
        } else if state == State::InPageAfterTitleAndId {
            if line.contains("<revision>") {
                rc += 1;
                state = State::InRevision;
            } else if line.contains("</page>") {
                if show_progress {
                    println!(
                        "{} (pid {}): {} revs '{}'",
                        pc,
                        pid.unwrap(),
                        rc,
                        title.as_ref().unwrap()
                    );
                    show_progress = false;
                }
                pc += 1;
                _rc_all += rc;
                state = State::PrePage;
                title = None;
                pid = None;
                text_buffer.clear();
            }
        } else if state == State::InRevision {
            if line.contains("<text") {
                state = State::InRevisionText;
                // Start accumulating text
                if let Some(end_tag) = line.find(">") {
                    text_buffer.push_str(&line[end_tag + 1..]);
                    text_buffer.push_str("\n");
                }
            } else if line.contains("</revision>") {
                state = State::InPageAfterTitleAndId;
            }
        } else if state == State::InRevisionText {
            text_buffer.push_str(&line);
            if line.contains("</text>") {
                state = State::InPageAfterTitleAndId;
                text_buffer.truncate(text_buffer.len() - 7);

                parse_page_wikitext(
                    &Page {
                        title: title.as_ref().unwrap().clone(),
                        ns,
                        id: pid,
                        rev_id: Some(-1), //None,
                        rev_contrib_id: None,
                        rev_text: text_buffer.clone(),
                    },
                    &mut pc,
                    &mut rc,
                );

                text_buffer.clear();
            } else {
                text_buffer.push_str("\n");
            }
        }
    }

    Ok(())
}
