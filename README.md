# Wikters

![Wikters Logo](wikters-logo.jpeg)

Wikters is a Rust project designed to read and process MediaWiki XML dumps specifically from the **English Wiktionary**. The project aims to efficiently parse and analyze the data stream, extracting relevant information from the pages.

It goes beyond the XML format to parse the wikitext format. Currently extracting just the language headings for pages that contain either or both English and "Translingual" sections.

The output will be in this form:
```
dictionary >>> Languages: English
free >>> Languages: English
thesaurus >>> Languages: English
encyclopedia >>> Languages: English
portmanteau >>> Languages: English
encyclopaedia >>> Languages: English
cat >>> Languages: Translingual, English
gratis >>> Languages: English
word >>> Languages: English
livre >>> Languages: English
book >>> Languages: English
pound >>> Languages: English
GDP >>> Languages: English
rain cats and dogs >>> Languages: English
```

## Features

- Parses XML data structured according to the MediaWiki export format.
- Extracts page titles, IDs, and revision IDs (currently not output).
- Outputs the names of all pages in the Wiktionary dump that include either an English or Translingual section, indicating which of the two languages are covered.
- Only deals with the main, definition namespace.

## Installation

To get started with Wikters, clone the repository and build the project:

```bash
git clone https://github.com/hippietrail/wikters.git
cd wikters
cargo build
```

## Usage

Wikters reads from `stdin`. To run the project and parse a MediaWiki XML dump, you can use one of the following commands:

- Using `cat` to pipe an XML dump:
  ```bash
  cat <path-to-xml-file> | cargo run
  ```

- Using `bzcat` to decompress directly from a bzip2 compressed XML dump file:
  ```bash
  bzcat <path-to-xml-file.bz2> | cargo run
  ```

Note that each language Wiktionary may implement its own format and as such Wikters only supports the English Wiktionary.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or features.
