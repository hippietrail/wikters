# Wikitext Parsing Refactor: v2 Architecture

## Problem with Original Approach

The original `l3_order_analyzer.rs` used linear scanning and eager section building:
- Scanned lines sequentially without proper structural awareness
- Built full section lists before analysis
- Mixed structural parsing with semantic classification
- Wasteful of computation when tools only need specific sections

## New Foundation: `wikitext_splitter.rs`

Following the approach used in MediaWiki PHP code, we now:

1. **Split once: `split_by_headings(wikitext) -> (headings, content_chunks)`**
   - `headings: Vec<Heading>` with `(level, text)` for each `==Heading==`
   - `content_chunks: Vec<String>` text between headings (len = headings.len() + 1)
   - `content_chunks[0]` is optional prolog before first heading
   - `content_chunks[i]` is text under `headings[i-1]`

2. **Work out nesting by analyzing heading levels**
   - No tree needed (yet)
   - Pure index math on the heading array
   - Lazy extraction of content

3. **Helper functions for common operations**
   - `find_language_section(headings, language)` → finds L2 section boundaries
   - `l3_headings_in_section(headings, start, end)` → gets all L3 heading indices
   - `content_for_heading(content_chunks, idx)` → gets content under a heading

## Key Design Decisions

### Structural vs Semantic
- **Structural (what we parse now):**
  - What's nested in what (based on `=` depth)
  - Where section boundaries are
  - Clean tree structure

- **Semantic (back burner):**
  - "Pron after Etym at same level" ≈ "Pron nested under Etym"
  - Language-specific patterns and rules
  - Will confuse us while building foundation

### Lazy Evaluation
- Each discovery tool works on its target slice of headings/content
- Only traverses what it needs
- No premature hierarchy building
- Content not extracted until actually needed

### Testing
- Core splitter has unit tests
- `l3_order_analyzer_v2.rs` validates on sample data
- Compare v1 vs v2 output to catch regressions

## Migration Path

1. Core splitter (`wikitext_splitter.rs`) - ✅ Done
2. Rewrite discovery tools one at a time using new foundation
3. When tools stabilize, consider building a real `Parser` trait
4. Later: proper tree representation if needed

## Current State

- `wikitext_splitter.rs` - Clean, tested, reusable
- `l3_order_analyzer_v2.rs` - Cleaner logic, uses splitter
- Original tools still work (keep for now, migrate gradually)

## Next Steps

1. Rewrite remaining discovery tools with splitter foundation
2. Run against full dump to verify patterns haven't changed
3. Once confident, replace old tools and remove v1
4. Document the wikitext_splitter API for future use

## Handoff Note

The key insight: don't try to build the perfect abstraction upfront. Split the wikitext once (cheap), let each tool walk the heading array (cheap index math), extract content on demand. This keeps tools simple, composable, and fast.
