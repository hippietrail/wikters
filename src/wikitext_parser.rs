use regex::Regex;
use crate::Page;

/// Parse wikitext from a page and output extracted data
/// Currently prints directly; future version will return structured Entry
pub fn parse_page_wikitext(
    page: &Page,
    page_num: &mut u64,
    section_num: &mut u64,
) {
    if page.ns.unwrap() != 0 {
        return;
    }

    let all_lang_headings_regex = Regex::new(r"(?m)^== ?([^=]*?) ?== *$\n").unwrap();
    let our_lang_headings_regex = Regex::new(r"(?m)^== ?(English|Translingual) ?== *$\n").unwrap();
    let mut lang_headings: Vec<String> = Vec::new();
    let mut languages: Vec<String> = Vec::new();

    for capture in all_lang_headings_regex.captures_iter(&page.rev_text) {
        if let (Some(heading), Some(lang)) = (capture.get(0), capture.get(1)) {
            lang_headings.push(heading.as_str().to_string());
            languages.push(lang.as_str().to_string());
        }
    }

    languages.retain(|lang| lang == "English" || lang == "Translingual");

    if languages.is_empty() {
        return;
    }

    // only count pages we don't reject
    *page_num += 1;

    // now split the text by the same regex
    let split_page_text = our_lang_headings_regex.split(&page.rev_text).collect::<Vec<&str>>();

    let _lang_sections_output_vec: Vec<String> = Vec::new();

    // skip the prologue before the first heading, usually contains {{also}}
    for (i, lang_sec_text) in split_page_text.iter().enumerate().skip(1) {
        *section_num += 1;

        let _lang_section_output = languages[i - 1].clone();

        // get everything after this heading
        let mut lang_sec_text = *lang_sec_text;
        // but keep only up to the next heading
        if let Some(heading) = all_lang_headings_regex.find(lang_sec_text) {
            lang_sec_text = &lang_sec_text[0..heading.start()];
        }

        let all_headings_regex = Regex::new(r"(?m)^==(?:=+) ?([^=]*?) ?==(?:=+) *$\n").unwrap();
        let our_headings_regex = Regex::new(r"(?m)^==(?:=+) ?(Noun) ?==(?:=+) *$\n").unwrap();
        let mut headings: Vec<String> = Vec::new();
        let mut heading_names: Vec<String> = Vec::new();

        for capture in all_headings_regex.captures_iter(lang_sec_text) {
            if let (Some(heading), Some(heading_name)) = (capture.get(0), capture.get(1)) {
                headings.push(heading.as_str().to_string());
                heading_names.push(heading_name.as_str().to_string());
            }
        }

        heading_names.retain(|heading_name| heading_name == "Noun");

        if heading_names.is_empty() {
            continue;
        }

        let split_section_text = our_headings_regex.split(&lang_sec_text).collect::<Vec<&str>>();

        let _heading_sections_output_vec: Vec<String> = Vec::new();

        for (j, section_text) in split_section_text.iter().enumerate().skip(1) {
            // let lump = section_text.replace("\n", "\\n").chars().take(72).collect::<String>();
            // let's find 'lump' a different way: let's iterate through the lines in section_text
            // and the first line to begin with { is the lump
            let mut lump = String::new();
            for line in section_text.lines() {
                if line.starts_with("{{en-") || line.starts_with("{{head|en|") || line.starts_with("{{head|mul|") {
                    lump = line.to_string();
                    break;
                }
            }
            println!("{}\t{}\t{}\t{}\t{}{}",
                page.title,
                languages[i - 1],
                j,
                if j == 0 { "⏺" } else { &heading_names[j - 1] },
                if j == 0 { "⏺" } else { "" },
                lump);
        }
    }
}
