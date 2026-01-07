/// Core wikitext splitter following MediaWiki PHP approach:
/// Split once into (headings, content) arrays, work out nesting by analyzing heading levels.
/// 
/// This keeps structure parsing clean and separate from semantic interpretation,
/// allows lazy extraction of only needed sections, and avoids reparsing.

use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Heading {
    pub level: usize,  // Number of = signs (2 = ==Language==, 3 = ===Etymology===, etc)
    pub text: String,  // Text between the = signs, trimmed
}

impl fmt::Display for Heading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}: {}", self.level, self.text)
    }
}

/// Split wikitext into headings and content chunks.
///
/// Returns: (headings, content_chunks)
/// - headings: Vec of (level, text) for each ==Heading==
/// - content_chunks: Vec of text blocks between headings (len = headings.len() + 1)
///
/// content_chunks[0] is the optional prolog before first heading
/// content_chunks[i] is the text under headings[i-1] (for i >= 1)
///
/// Example:
/// ```
/// Some prologue
/// ==English==
/// Etymology text
/// ===Etymology===
/// Noun definition
/// ====Noun====
/// ```
/// Returns:
/// - headings: [(2, "English"), (3, "Etymology"), (4, "Noun")]
/// - content_chunks: ["Some prologue\n", "Etymology text\n", "Noun definition\n", ""]
pub fn split_by_headings(wikitext: &str) -> (Vec<Heading>, Vec<String>) {
    let mut headings = Vec::new();
    let mut content_chunks = Vec::new();
    let mut current_content = String::new();

    for line in wikitext.lines() {
        let trimmed = line.trim();
        if let Some(heading) = parse_heading(trimmed) {
            // We hit a heading - save current content and record heading
            content_chunks.push(current_content);
            current_content = String::new();
            headings.push(heading);
        } else {
            // Regular content line
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    // Push final content chunk
    content_chunks.push(current_content);

    (headings, content_chunks)
}

/// Try to parse a line as a heading. Returns Some(Heading) or None.
fn parse_heading(line: &str) -> Option<Heading> {
    let trimmed = line.trim();
    
    // Count leading = signs
    let leading = trimmed.chars().take_while(|c| *c == '=').count();
    
    // Must have at least 2
    if leading < 2 {
        return None;
    }
    
    // Count trailing = signs
    let trailing = trimmed.chars().rev().take_while(|c| *c == '=').count();
    
    // Leading and trailing must match, and there must be text between
    if leading != trailing || leading * 2 >= trimmed.len() {
        return None;
    }
    
    // Extract text between = signs
    let text = trimmed[leading..trimmed.len() - trailing]
        .trim()
        .to_string();
    
    Some(Heading {
        level: leading,
        text,
    })
}

/// Find the byte range (start_idx, end_idx) of headings that belong to a language section.
///
/// Returns (start, end) such that headings[start..end] are in the language section,
/// and content_chunks[start..end+1] are the corresponding content.
pub fn find_language_section(headings: &[Heading], language: &str) -> Option<(usize, usize)> {
    // Find the L2 heading matching this language
    let start = headings.iter().position(|h| h.level == 2 && h.text.contains(language))?;

    // Find the next L2 heading (or end of array)
    let end = headings[start + 1..]
        .iter()
        .position(|h| h.level == 2)
        .map(|p| p + start + 1)
        .unwrap_or(headings.len());

    Some((start, end))
}

/// Extract all L3 headings within a section (between start and end indices).
pub fn l3_headings_in_section(headings: &[Heading], start: usize, end: usize) -> Vec<usize> {
    headings[start..end]
        .iter()
        .enumerate()
        .filter_map(|(i, h)| if h.level == 3 { Some(start + i) } else { None })
        .collect()
}

/// Get the content for a given heading index (the text between that heading and the next one).
pub fn content_for_heading(content_chunks: &[String], heading_idx: usize) -> &str {
    // content_chunks[heading_idx + 1] is the content under headings[heading_idx]
    content_chunks
        .get(heading_idx + 1)
        .map(|s| s.as_str())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_basic() {
        let wikitext = "Prolog\n==English==\nSome text\n===Etymology===\nEtym text";
        let (headings, content) = split_by_headings(wikitext);

        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].level, 2);
        assert_eq!(headings[0].text, "English");
        assert_eq!(headings[1].level, 3);
        assert_eq!(headings[1].text, "Etymology");

        assert_eq!(content.len(), 3); // prolog, then content under English, then under Etymology
        assert!(content[0].contains("Prolog"));
        assert!(content[1].contains("Some text"));
        assert!(content[2].contains("Etym text"));
    }

    #[test]
    fn test_find_language_section() {
        let headings = vec![
            Heading { level: 2, text: "English".to_string() },
            Heading { level: 3, text: "Etymology".to_string() },
            Heading { level: 2, text: "French".to_string() },
            Heading { level: 3, text: "Étymologie".to_string() },
        ];

        let (start, end) = find_language_section(&headings, "English").unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 2);

        let (start, end) = find_language_section(&headings, "French").unwrap();
        assert_eq!(start, 2);
        assert_eq!(end, 4);
    }

    #[test]
    fn test_l3_headings() {
        let headings = vec![
            Heading { level: 2, text: "English".to_string() },
            Heading { level: 3, text: "Etymology".to_string() },
            Heading { level: 4, text: "Noun".to_string() },
            Heading { level: 3, text: "Pronunciation".to_string() },
        ];

        let l3s = l3_headings_in_section(&headings, 0, 4);
        assert_eq!(l3s.len(), 2);
        assert_eq!(l3s[0], 1); // Etymology at index 1
        assert_eq!(l3s[1], 3); // Pronunciation at index 3
    }
}
