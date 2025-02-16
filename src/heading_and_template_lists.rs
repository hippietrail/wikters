pub const HEADING_WHITELIST: [&str; 33] = [
    "Adjective",
    "Adverb",
    "Article",
    "Alternative forms",
    "Conjunction",
    "Etymology",
    "Etymology 1",
    "Etymology 2",
    "Etymology 3",
    "Etymology 4",
    "Etymology 5",
    "Etymology 6",
    "Etymology 7",
    "Etymology 8",
    "Etymology 9",
    "Etymology 10",
    "Etymology 11",
    "Etymology 12",
    "Etymology 13",
    "Etymology 14",
    "Etymology 15",
    "Etymology 16",
    "Etymology 17",
    "Etymology 18",
    "Etymology 19",
    "Etymology 20",
    "Noun",
    "Particle",
    "Preposition",
    "Pronoun",
    "Pronunciation",
    "Proper noun",
    "Verb",
];

pub const HEADING_BLACKLIST: [&str; 85] = [
    // "Abbreviations", // keep for now
    "Additional notes",
    "Alternative pronunciation", "Alternative Pronunciation",
    "Alternative spellings",
    "Anagrams",
    "Antonyms",
    "Attestations",
    "Circumfix",
    "Collocations",
    "Combining forms", "Combining form",
    "Comeronyms",
    "Common nouns",
    "Composition",
    "Conjugation",
    // "Contraction", // keep for now
    "Coordinate terms",
    "Cuneiform sign",
    "Declension",
    "Derivations",
    "Derivative words",
    "Derived characters", "Derived Characters",
    "Derived forms",
    "Derived glyphs",
    "Derived signs",
    "Derived terms",
    "Derived words",
    "Descendants",
    "Description",
    "Design",
    "Determiner", // keep for now
    "Diacritic",
    "Diacritical mark",
    "Dialects",
    // "Etymology", // keep because there can be multiple
    "Example", "Examples",
    "External links",
    "Formation",
    "Forms",
    "Further reading",
    "Gallery",
    "Glyph origin",
    "Han character",
    "Historical notes",
    "Holonyms",
    "Hypernyms",
    "Hyponyms",
    "Idiom",
    "Interfix",
    "Infix",
    "Letter",
    "Ligature",
    "Links",
    "Meronyms",
    "Multiple parts of speech",
    "Note", "Notes",
    "Number",
    "Numeral",
    "Origin",
    "Other names",
    "Parasynonyms",
    "Paronyms",
    // "Particle", // keep for now
    "Phrase",
    // "Prefix", // keep for now
    // "Prepositional phrase", // keep for now
    "Production",
    "Pron",
    // "Pronunciation", // keep because homophones are in here
    "Pronunciation notes",
    // "Proper nouns", // not sure about this one
    // "Proverb", // not sure about this one
    "Punctuation mark",
    "Quotations",
    "References",
    "Related characters",
    "Related forms",
    "Related symbols",
    "Related terms",
    "See also",
    "Statistics",
    // "Suffix", // keep for now
    "Symbol",
    "Symbol origin",
    "Symbols",
    "Synonyms",
    "Translations",
    "Trivia",
    "Troponyms",
    "Unrelated terms",
    "Usage notes",
];

pub const TEMPLATE_WHITELIST: [&str; 38] = [
    "alternative spelling of", "alt spelling of",
    "archaic spelling of",
    "censored spelling of",
    "dated spelling of",
    "deliberate misspelling of",
    // en-early modern spelling of
    // filter-avoidance spelling of
    "informal spelling of",
    "intentional misspelling of",
    "less common spelling of",
    "misconstruction of",
    "misspelling of",
    "nonstandard spelling of",
    "obsolete spelling of",
    "pronunciation spelling of",
    "rare spelling of",
    "standard spelling of",
    "uncommon spelling of",

    "alternative case form of",
    "alternative form of",
    "archaic form of",
    "obsolete form of",
    "uncommon form of",

    "alt form",
    "alt sp",

    "alt", "alter",

    "abbreviation of",
    "abbr of",
    "acronym of",
    "infl of",
    "initialism of",
    "init of",
    "past participle of",
    "plural of",
    "synonym of",
    "syn of",

    "en-comparative of",
    "en-superlative of",
];
// grey templates
// --------------
// head
// en-adj
// en-adv
// en-interj
// en-noun
// en-PP
// en-pref
// en-proper noun
// en-proper-noun
// en-prop
// en-verb
//
// prefix
// suffix
//
// af
// compound
// der
// given name
// place
// q
// qualifier
// sense
// surname
// syn
// synonyms
pub const TEMPLATE_BLACKLIST: [&str; 68] = [
    "Han char", "Han ref",

    "l", "link",
    "lb", "label",

    "t-check",
    "t-needed",
    "t",
    "t+",
    "t+check",
    "tt",
    "tt+",

    "...",
    "anagrams",
    "audio",
    "bor",
    "C", "topics", "c",
    "cite-book",
    "cln", "catlangname",
    "cog",
    "col", "col2", "col3", "col4",
    "comcatlite",
    "enPR",
    "etystub",
    "inh",
    "IPA", "IPAchar",
    "m", "mention",
    "multiple image", "multiple images",
    "nbsp",
    "pedia",
    "quote-book", "quote-book ",
    "quote-gloss",
    "quote-journal",
    "quote-text",
    "quote-web",
    "rfe",
    "rhymes",
    "seeCites",
    "specieslite",
    "taxlink", "taxfmt", "taxoninfl", "taxon",
    "trans-top", "trans-bottom", "trans-see", "trans-top-also",
    "ux",
    "vern",
    "was wotd",
    "wikipedia", "w", "wp",
    "wikispecies",

    "R:",
    "RQ:",
    "U:",
];