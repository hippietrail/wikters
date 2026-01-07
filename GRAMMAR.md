# Wikters Grammar Sketches

## MediaWiki XML Dump Format (PEG)

Based on the state machines in `regex_reader.rs` and `string_ops_reader.rs`.
This is the **constrained MediaWiki dump XML** format used for Wiktionary exports.

```
dump = siteinfo? page*

page = "<page>" line_break
       page_header
       revision+
       "</page>" line_break

page_header = title ns? id additional_fields*

title = "<title>" text "</title>" line_break
ns = "<ns>" number "</ns>" line_break
id = "<id>" number "</id>" line_break
additional_fields = ~"<[^>]+>" text? "<[^/>]+>" line_break

revision = "<revision>" line_break
           revision_content
           "</revision>" line_break

revision_content = (id | timestamp | contributor | comment | model | format | text)*

text = "<text" ~"[^>]*" ">" wikitext "</text>" line_break

wikitext = ~"(?s).*?(?=</text>)"

siteinfo = "<siteinfo>" ~"(?s).*?" "</siteinfo>" line_break

line_break = "\n"
text = ~"[^<]*"
number = ~"\d+"
```

**Key assumptions:**
- Each structural tag (`<page>`, `<title>`, `</page>`) is on its own line (no tags broken mid-line except in `<text>` content)
- `<text>` can contain anything (wikitext) until `</text>`
- We only care about `title`, `ns`, `id`, and `text` tags


## Wikitext Article Format (PEG) — EXPLORATORY

Inferred from `wikitext_parser.rs`. This is **speculative** and incomplete.

```
article = preamble lang_section*

preamble = ~"(?s)(?=^==)"    ; Skip until first heading

lang_section = lang_heading section_content

lang_heading = "==" ws lang_name ws "==" ws line_break
lang_name = "English" | "Translingual" | other_language_name

section_content = pos_section* closing_heading?

pos_section = pos_heading pos_body

pos_heading = "===" ws pos_name ws "===" ws line_break
pos_name = "Noun" | "Verb" | "Adjective" | "Adverb" | ...

pos_body = template_or_text*

template_or_text = (noun_template | verb_template | other_template | text)*

noun_template = "{{en-noun" ~"[^}]*" "}}"
             | "{{head|en|noun" ~"[^}]*" "}}"
             | "{{head|mul|noun" ~"[^}]*" "}}"

verb_template = "{{en-verb" ~"[^}]*" "}}"
             | "{{head|en|verb" ~"[^}]*" "}}"

other_template = "{{" template_name ~"[^}]*" "}}"

closing_heading = "=" line_break  ; Any heading (back to lang level or end)

ws = " "*
text = ~"[^\n<{]+"
line_break = "\n"
```

**Caveats:**
- Only extracts the first template in each POS section (line starting with `{{`)
- Ignores definition text, examples, pronunciations
- Heading nesting is **complex** (varies by article structure) — current code just looks for POS names
- Templates can have nested `{{}}` (currently not handled recursively)
- Many POS variants not listed (Etymology, Alternative forms, etc.)
- Language names beyond English/Translingual are parsed but filtered out


## Notes for Future Parsers

1. **State machine vs. recursive descent**: The current XML parsing uses a flat state machine (appropriate for line-by-line). The wikitext parsing could benefit from recursive descent (sections contain subsections, templates nest).

2. **Lazy extraction**: Both grammars skip irrelevant content early. This is critical for performance on 1.3GB dumps.

3. **Template complexity**: Wikitext templates use `|` for parameters and can nest. A proper parser would need to handle:
   ```
   {{en-noun|s|head=foo|extra={{nested}}}}
   ```

4. **Language handling**: Current code filters for English/Translingual after parsing all languages. Could optimize by skipping non-target languages entirely.

5. **Ambiguity**: Wikitext relies on wiki conventions (not strict grammar), so formal parsing is inherently lossy.
