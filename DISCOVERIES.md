# Wiktionary Wikitext Format Discoveries & Analysis

Empirical analysis of enwiktionary-20251101 dump (50,000 page sample).

## Executive Summary

English sections (n=172k, from 500k page sample) show clear patterns:

- **11% PosOnly** - Minimal entries, no Etymology/Pronunciation
- **9% EtymFlatThenPronFlat** - Most common structured entries
- **5% EtymOnly** - Etymology without Pronunciation metadata
- **3% PronOnly** - Standalone Pronunciation
- **1% PronFlatThenEtymFlat** - Reverse order (rare)
- **0.4% EtymWithNestedPron** - Etymology with L4:Pronunciation inside
- **~0.1% edge cases** - No L3 sections, no structured content, etc.
- **0% PronWithNestedEtym** - This pattern doesn't exist in the corpus

**Key insight**: Two dominant patterns (PosOnly at 11%, EtymFlatThenPronFlat at 9%) handle 20% of entries. Rest are variations. Parser must handle nested and flat forms, but nesting is rare (0.4%).

## Detailed Findings by Pattern

### L3 Section Ordering (of pages with English sections, n≈172k, on 500k page sample)

| Pattern | Count | % | Examples | Structure |
|---------|-------|---|----------|-----------|
| **PosOnly** | 56,658 | 11% | apples and pears, A 1, aard-vark | Only L3:POS, no Etymology/Pronunciation |
| **EtymFlatThenPronFlat** | 48,059 | 9% | dictionary, thesaurus, encyclopedia | L3:Etymology → L3:Pronunciation (sequential) |
| **EtymOnly** | 28,609 | 5% | Pope Julius, portmanteau word, ab- | L3:Etymology only (may have L4:Pronunciation inside) |
| **PronOnly** | 16,556 | 3% | GDP, pies, GNU FDL, current events | L3:Pronunciation only |
| **PronFlatThenEtymFlat** | 5,120 | 1% | free, portmanteau, cat, word | L3:Pronunciation → L3:Etymology (reverse order) |
| **EtymWithNestedPron** | 1,950 | 0.4% | A, raven, July, minute | L3:Etymology with L4:Pronunciation inside |
| **Other (no_etym_pron_pos)** | 427 | 0.1% | ik, ttyl, RTFM, YMMV | Edge case: no etymology/pronunciation/POS |
| **Other (no_l3)** | 50 | 0.01% | abnodate, abnodation | Edge case: no L3 sections at all |
| **EtymFlatThenPronNested** | 1 | 0.002% | de | Very rare: unclear what this means |
| **PronFlatThenEtymNested** | 1 | 0.002% | pull-up | Very rare: unclear what this means |
| **PronWithNestedEtym** | 0 | 0% | - | **Does not exist in corpus** |

### L4 Heading Patterns (nested under L3, top 20)

Most L4 sections are content metadata, not structural dividers:

| L4 Type | Count | Parent L3 (Example) | Notes |
|---------|-------|-------------------|-------|
| **Translations** | 15,590 | Noun, Verb, Adjective | Most common L4 section |
| **Derived terms** | 10,904 | Noun, Adjective, Verb | Second most common |
| **Noun** | 5,671 | Etymology | POS nested under Etymology |
| **Related terms** | 4,954 | Noun, Adjective | Content subsection |
| **Synonyms** | 4,296 | Noun, Adjective, Verb | Content subsection |
| **Verb** | 2,955 | Etymology | POS nested under Etymology |
| **Pronunciation** | 1,350 | Etymology | **Per-etymology pronunciation details** |

**Critical finding**: ====Pronunciation==== under ===Etymology=== (1,350 occurrences) is significant, not rare. Each etymology can have its own pronunciation specifications (different for homographs/different word senses).

### Pronunciation Nesting Patterns

Distinguishes top-level vs nested Pronunciation:

| Pattern | Count | % | Meaning |
|---------|-------|---|---------|
| **Neither** | 32,279 | 65% | No Pronunciation section at all |
| **TopLevelL3** | 15,155 | 30% | ===Pronunciation=== shared across etymologies |
| **NestedL4UnderEtymology** | 776 | 1.5% | ====Pronunciation==== per etymology |
| **Both** | 29 | 0.06% | Complex pages with both patterns |

**Ratio**: Top-level Pronunciation is ~20x more common than nested.

### Visual Examples by Type

#### "free" - PronunciationBeforeEtymology (4%)
```
==English==
===Pronunciation===
  * Shared pronunciation info

===Etymology 1===
  From ...
====Adjective====
  Definition 1

===Etymology 2===
  From ...
====Adjective====
  Definition 2
```

Key: Pronunciation shared across homographs; each has L4:Adjective (not L3).

#### "dictionary" - EtymologyBeforePronunciation (22%)
```
==English==
===Etymology===
From ...

===Pronunciation===
* IPA info

===Noun===
Definition
```

Key: Sequential L3 sections, flat structure, no nesting.

#### "July" - OnlyEtymology (5%, with nested Pronunciation)
```
==English==
===Etymology 1===
====Pronunciation====
  Pronunciation for sense 1
====Proper noun====
  Definition 1

===Etymology 2===
====Pronunciation====
  Pronunciation for sense 2
====Proper noun====
  Definition 2
```

Key: Each Etymology has own L4:Pronunciation and L4:POS.

#### "apples and pears" - PosOnly (4%)
```
==English==
===Noun===
Definition...
```

Key: Minimal structure, no Etymology/Pronunciation at all.

## Parser Design Implications

### State Machine Approach (Validated)

1. **Expect variable nesting levels**: Both L3→L3 and L3→L4 exist
2. **Don't assume section order**: Etymology-first and Pronunciation-first both common
3. **Shared sections exist**: Pronunciation can apply to multiple etymologies/senses
4. **Content sections are deeper**: Translations, Derived terms are L4+ under POS, not dividers
5. **Lazy extraction works**: Skip full hierarchy parsing, extract on-demand by heading level

### What NOT to do

❌ Assume Etymology is always the top-level divider  
❌ Assume POS always nests under Etymology  
❌ Build a rigid recursive hierarchy parser  
❌ Parse all 1800+ unique heading patterns uniformly  

### What TO do

✅ State machine: track current level, collect section metadata  
✅ Lazy extraction: when asked for "Noun", find nearest L3/L4:Noun, extract until next section  
✅ Flexible grouping: Pronunciation can be L3 or L4, group by function not level  
✅ Early stopping: skip to relevant sections fast using heading-level markers  

## Discovery Tools

All stream from bzcat without disk decompression. Each stores 3-4 example page titles for lookup:

- **section_structure_inspector**: High-level pattern analysis
- **etymology_pronunciation_analyzer**: Detailed nesting with examples
- **homograph_pattern_detector**: Classification into architecture types
- **level4_heading_analyzer**: L4 patterns and parent-child relationships
- **pronunciation_nesting_analyzer**: Top-level vs nested Pronunciation distinction
- **l3_order_analyzer**: Section ordering patterns with example pages
- **dump_raw_sections**: Extract raw wikitext for manual inspection

Usage:
```bash
# Analyze patterns
bzcat dump.xml.bz2 | cargo run --release --bin l3_order_analyzer --limit 50000 --output-examples "-"

# Look up examples
bzcat dump.xml.bz2 | cargo run --release --bin dump_raw_sections --title-filter "free"
```

## Known Gotchas

1. **Numbered sections**: ===Etymology 1===, ===Etymology 2=== require normalization for grouping
2. **Case sensitivity**: Section type matching must handle "Pronunciation", "pronunciation", etc
3. **Non-article namespaces**: ~55% of pages in dump are not in namespace 0 (articles)
4. **Edge cases exist**: Some pages have 5-10+ etymologies (words like "a", "be", "do")
5. **Minimal entries**: Some entries skip all metadata sections

## Future Investigation

- [ ] Compare English patterns to other languages (German, French, Chinese)
- [ ] Analyze Translingual sections separately
- [ ] Study rare patterns in detail (the 0.06% with both Pronunciation patterns)
- [ ] Profile streaming performance vs. full parsing
- [ ] Validate parser against 100k+ page sample
