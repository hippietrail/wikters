# Wikters

![Wikters Logo](wikters-logo.jpeg)

Wikters is a Rust project designed to read and process MediaWiki XML dumps specifically from the **English Wiktionary**. The project aims to efficiently parse and analyze the data stream, extracting relevant information from the pages.

It goes beyond the XML format to parse the wikitext format. Currently extracting just the language headings for pages that contain either or both English and "Translingual" sections.

The output will be in this form:
```
encyclopaedia
  English
  Etymology
  Pronunciation
  Noun

cat
  English
  Pronunciation
  Etymology 1
    Alternative forms
    Noun
    Verb
  Etymology 2
    Noun
    Verb
  Etymology 3
    Noun
    Adjective
```

Or if you specify `-x` on the commandline it will output a lightweight XML format:
```xml
  <p n="7" pid="36" rid="83729202">
    <t>cat</t>
    <s n="8" l="English">
      <x>
          Pronunciation
          Etymology 1
            Alternative forms
            Noun
            Verb
          Etymology 2
            Noun
            Verb
          Etymology 3
            Noun
            Adjective</x>
    </s>
  </p>
```
## Features

- Parses XML data structured according to the MediaWiki export format.
- Extracts page titles, IDs, and revision IDs (currently not output).
- Outputs the names of all pages in the Wiktionary dump that include either an English or Translingual section, indicating which of the two languages are covered.
- Only deals with the main, definition namespace.

## Getting the English Wiktionary XML Dump

To obtain the English Wiktionary XML dump file, visit the following webpage: [Wikimedia Downloads](https://dumps.wikimedia.org/backup-index.html). Look in the huge list for the `enwiktionary` entry, which looks like this:
> 2025-02-08 05:12:28 [enwiktionary](https://dumps.wikimedia.org/enwiktionary/20250201): Dump complete

Follow its link, currently [https://dumps.wikimedia.org/enwiktionary/20250201](https://dumps.wikimedia.org/enwiktionary/20250201). 

From there, find the `pages-articles` dump file link, currently [https://dumps.wikimedia.org/enwiktionary/20250201/enwiktionary-20250201-pages-articles.xml.bz2](https://dumps.wikimedia.org/enwiktionary/20250201/enwiktionary-20250201-pages-articles.xml.bz2). The current file is 1.3GB in bzip2 format.

(The multistream versions should also work but are slightly larger, currently 1.6GB.)

If you find that a new dump is in progress and these links are grayed out, there will be a link to the previous dump, allowing you to get a slightly older version of the file. For example:
> [Last dumped on 2025-01-20](https://dumps.wikimedia.org/enwiktionary/20250120/)

There is also a ["Index of /enwiktionary/latest/"](https://dumps.wikimedia.org/enwiktionary/latest/) page with a different format that is just a straight list of links to the individual dump files.

The dumps are updated on the 1st and 20th of every month.

## Installation

To get started with Wikters, clone the repository and build the project:

```bash
git clone https://github.com/hippietrail/wikters.git
cd wikters
cargo build
```

## Usage

Wikters reads from `stdin`. To run the project and parse a MediaWiki XML dump, you can use the following command:

- Use `bzcat` to decompress directly from a bzip2 compressed XML dump file through a pipe:
  ```bash
  bzcat <path-to-xml-file.bz2> | cargo run
  ```
If you have plenty of storage you can decompress the `.bz2` file first into an `.xml` file and it might run slightly faster. In that case:

- Use `cat` to pipe the raw XML dump:
  ```bash
  cat <path-to-xml-file> | cargo run
  ```

Note that each language Wiktionary may implement its own format and as such Wikters only supports the English Wiktionary.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or features.
