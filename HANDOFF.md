# Handoff Notes for Next Session

## Current State (as of Jan 8 2026)

### Architecture
- **Core module**: `src/wikitext_splitter.rs` - Clean, tested foundation
  - Splits wikitext once: `(headings: Vec<Heading>, content_chunks: Vec<String>)`
  - Headings encode tree structure implicitly via level numbers
  - Lazy extraction: tools only traverse what they need
  - No premature tree building

- **Discovery tools**: Stream from XML via `PageSource` trait
  - `l3_order_analyzer_v2.rs` - Rewritten using splitter (clean, correct)
  - Original tools still exist but v2 is the better approach
  - Each tool works on language section slice of headings/content

- **Design principle**: Structural parsing (what's nested in what) is separate from semantic interpretation (what patterns mean)

### Key Findings (10k page sample)

**Dump location**: `/Volumes/DYNABOOK/wiki/enwiktionary-20260101-pages-articles.xml.bz2`

**Sample results**:
- 10,000 pages read
- 5,337 have English sections (53.37%)
- Of English entries:
  - 57.43% EtymFlatThenPronFlat (most common)
  - 13.36% PronFlatThenEtymFlat
  - 8.84% PronOnly
  - 8.68% PosOnly
  - 6.43% EtymOnly
  - 5.15% EtymWithNestedPron (nested Pronunciation under Etymology)
  - 0.12% edge cases

**Important**: The nesting detection improved with structural parsing (5.15% vs old 0.4%)

### Running Analysis

```bash
# Quick test (10k pages)
bzcat /Volumes/DYNABOOK/wiki/enwiktionary-20260101-pages-articles.xml.bz2 | \
  cargo run --release --bin l3_order_analyzer_v2 -- --limit 10000

# Full analysis (takes ~5-10 min)
bzcat /Volumes/DYNABOOK/wiki/enwiktionary-20260101-pages-articles.xml.bz2 | \
  cargo run --release --bin l3_order_analyzer_v2
```

### To Do Next

- [ ] Migrate remaining discovery tools to use `wikitext_splitter` foundation
- [ ] Run full dump analysis to confirm 10k patterns scale
- [ ] Update DISCOVERIES.md with new findings
- [ ] Consider removing old tools once migration complete
- [ ] Add more language support (if needed)
- [ ] Build parser trait/library interface (when tools stabilize)

### Testing

- Unit tests in `wikitext_splitter.rs` ✓
- Sample XML in `test_sample.xml` with "run", "test", "bank" examples
- v1 and v2 produce identical results on same data ✓

### Key Insights

1. **Nesting is implicit in heading levels** - Don't build trees, just walk the array
2. **Lazy evaluation matters** - Different tools need different slices of the hierarchy
3. **Structural vs semantic** - Keep parsing clean, defer interpretation
4. **MediaWiki PHP pattern** - Split once, work out nesting by analyzing levels

### Files to Know

- `src/wikitext_splitter.rs` - Foundation (keep this clean)
- `src/bin/l3_order_analyzer_v2.rs` - Best current implementation
- `DISCOVERIES.md` - Original findings (needs updating with new data)
- `REFACTOR_NOTES.md` - Architecture decisions
- `test_sample.xml` - Good for quick testing without big dump

### Dump Logistics

- Dumps at: `/Volumes/DYNABOOK/wiki/enwiktionary-YYYYMMDD-pages-articles.xml.bz2`
- Latest (as of Jan 2026): `enwiktionary-20260101-pages-articles.xml.bz2` (1.4GB)
- Format: bzip2 compressed XML
- Tool: `bzcat` pipes directly without decompression
- Use `--limit N` for quick testing (10k is fast, ~3-5 sec)

### Notes for Debugging

- If "no_english" entries spike, check if language filtering is working
- The percentage output now shows both "% of English" and "% of all dump"
- Run sample XML first if making parser changes: `cat test_sample.xml | cargo run --release --bin l3_order_analyzer_v2`
- Remember: headings are at the heading_idx, content is at content_chunks[heading_idx + 1]
