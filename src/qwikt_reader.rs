use std::error::Error;
use std::io::Read;
use std::fmt;

use crate::{Page, PageSource};

#[derive(Debug)]
struct QwiktError(String);

impl fmt::Display for QwiktError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for QwiktError {}

#[derive(Debug, Clone, Copy)]
struct Position {
    off: usize,
    line: usize,
    col: usize,
}

impl Position {
    fn new() -> Self {
        Position {
            off: 0,
            line: 1,
            col: 1,
        }
    }

    fn advance(&mut self, byte: u8) {
        self.off += 1;
        if byte == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
    }
}

struct StreamReader<R: Read> {
    reader: R,
    position: Position,
}

impl<R: Read> StreamReader<R> {
    fn new(reader: R) -> Self {
        StreamReader {
            reader,
            position: Position::new(),
        }
    }

    fn read_byte(&mut self) -> Result<u8, Box<dyn Error>> {
        let mut buf = [0u8; 1];
        match self.reader.read(&mut buf)? {
            1 => {
                self.position.advance(buf[0]);
                Ok(buf[0])
            }
            _ => Err(Box::new(QwiktError("Unexpected EOF".to_string()))),
        }
    }

    fn match_exact(&mut self, expected: &[u8]) -> Result<(), Box<dyn Error>> {
        for &expected_byte in expected {
            let actual = self.read_byte()?;
            if actual != expected_byte {
                return Err(Box::new(QwiktError(
                    format!(
                        "Mismatch at byte {} (line {}, col {}): expected {:?}, got {:?}",
                        self.position.off,
                        self.position.line,
                        self.position.col,
                        expected_byte as char,
                        actual as char
                    )
                )));
            }
        }
        Ok(())
    }

    fn read_until(&mut self, delimiter: u8) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut result = Vec::new();
        loop {
            let byte = self.read_byte()?;
            if byte == delimiter {
                return Ok(result);
            }
            result.push(byte);
        }
    }
}

pub struct QwiktReader<R: Read> {
    stream: StreamReader<R>,
    initialized: bool,
}

impl<R: Read> QwiktReader<R> {
    pub fn new(reader: R) -> Self {
        QwiktReader {
            stream: StreamReader::new(reader),
            initialized: false,
        }
    }

    fn init_header(&mut self) -> Result<(), Box<dyn Error>> {
        // mediawiki element with its attrs
        self.stream.match_exact(b"<mediawiki xmlns=\"http://www.mediawiki.org/xml/export-0.11/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://www.mediawiki.org/xml/export-0.11/ http://www.mediawiki.org/xml/export-0.11.xsd\" version=\"0.11\" xml:lang=\"")?;
        let _lang_code = self.stream.read_until(b'"')?;
        self.stream.match_exact(b">\n")?;

        // siteinfo lasts until the first page
        self.stream.match_exact(b"  <siteinfo>\n    <sitename>")?;
        let _sitename = self.stream.read_until(b'<')?;
        self.stream.match_exact(b"/sitename>\n    <dbname>")?;

        let _dbname = self.stream.read_until(b'<')?;
        self.stream.match_exact(b"/dbname>\n    <base>")?;

        let _base_url = self.stream.read_until(b'<')?;

        self.stream.match_exact(b"/base>\n    <generator>MediaWiki ")?;

        let _gen_ver = self.stream.read_until(b'<')?;

        self.stream.match_exact(b"/generator>\n    <case>case-sensitive</case>\n    <namespaces>\n")?;

        loop {
            self.stream.match_exact(b"    ")?;
            // Check if we've reached the closing tag
            let first_byte = self.stream.read_byte()?;

            if first_byte == b'<' {
                self.stream.match_exact(b"/namespaces>\n")?;
                break;
            }

            self.stream.match_exact(b" <namespace key=\"")?;
            let _ns_key = self.stream.read_until(b'"')?;
            self.stream.match_exact(b" case=\"")?;
            let _ns_case = self.stream.read_until(b'"')?;

            let next_byte = self.stream.read_byte()?;

            if next_byte == b'>' {
                let _ns_name_data = self.stream.read_until(b'<')?;
                self.stream.match_exact(b"/namespace>\n")?;
            } else if next_byte == b' ' {
                self.stream.match_exact(b"/>\n")?;
            } else {
                return Err(Box::new(QwiktError(format!("Expected '>' or ' ', got {:?}", next_byte as char))));
            }
        }

        self.stream.match_exact(b"  </siteinfo>\n")?;
        Ok(())
    }
}

impl<R: Read> PageSource for QwiktReader<R> {
    fn next_page(&mut self) -> Result<Option<Page>, Box<dyn Error>> {
        // Initialize header on first call
        if !self.initialized {
            self.init_header()?;
            self.initialized = true;
        }

        let first_byte = match self.stream.read_byte() {
            Ok(b) => b,
            Err(_) => return Ok(None),
        };
        
        if first_byte == b'<' {
            return Ok(None);
        }

        self.stream.match_exact(b" <page>\n    <title>")?;

        let title_bytes = self.stream.read_until(b'<')?;
        let title = String::from_utf8_lossy(&title_bytes).into_owned();

        self.stream.match_exact(b"/title>\n    <ns>")?;
        let ns_bytes = self.stream.read_until(b'<')?;
        let ns = String::from_utf8_lossy(&ns_bytes).parse::<i32>().ok();

        self.stream.match_exact(b"/ns>\n    <id>")?;
        let id_bytes = self.stream.read_until(b'<')?;
        let id = String::from_utf8_lossy(&id_bytes).parse::<i32>().ok();
        
        self.stream.match_exact(b"/id>\n    <re")?;

        let byte = self.stream.read_byte()?;

        if byte == b'd' {
            self.stream.match_exact(b"irect title=\"")?;
            let _redirect_title = self.stream.read_until(b'"')?;
            self.stream.match_exact(b" />\n    <rev")?;
        } else if byte != b'v' {
            return Err(Box::new(QwiktError(format!("Expected 'd' or 'v', got {:?}", byte as char))));
        }

        self.stream.match_exact(b"ision>\n      <id>")?;
        let _rev_id = self.stream.read_until(b'<')?;

        self.stream.match_exact(b"/id>\n      <")?;

        let byte = self.stream.read_byte()?;
        if byte == b'p' {
            self.stream.match_exact(b"arentid>")?;
            let _parent_id = self.stream.read_until(b'<')?;
            self.stream.match_exact(b"/parentid>\n      <timestamp>")?;
        } else if byte == b't' {
            self.stream.match_exact(b"imestamp>")?;
        } else {
            return Err(Box::new(QwiktError(format!("Expected 'p' or 't', got {:?}", byte as char))));
        }

        let _timestamp = self.stream.read_until(b'<')?;
        self.stream.match_exact(b"/timestamp>\n      <contributor")?;

        let byte = self.stream.read_byte()?;
        let rev_contrib_id: Option<i32> = if byte == b' ' {
            self.stream.match_exact(b"deleted=\"deleted\" />\n      <")?;
            None
        } else {
            self.stream.match_exact(b"\n        <")?;

            let mut contrib_id = None;
            let byte = self.stream.read_byte()?;

            // contributor - username+id or IP
            if byte == b'u' {
                self.stream.match_exact(b"sername>")?;
                let _username = self.stream.read_until(b'<')?;

                self.stream.match_exact(b"/username>\n        <id>")?;
                let contrib_id_bytes = self.stream.read_until(b'<')?;
                contrib_id = String::from_utf8_lossy(&contrib_id_bytes).parse::<i32>().ok();
                self.stream.match_exact(b"/id>\n      </contributor>\n      <")?;
            } else if byte == b'i' {
                self.stream.match_exact(b"p>")?;
                let _ip = self.stream.read_until(b'<')?;
                self.stream.match_exact(b"/ip>\n      </contributor>\n      <")?;
            } else {
                return Err(Box::new(QwiktError(format!("Expected 'u' or 'i', got {:?}", byte as char))));
            }
            contrib_id
        };

        // optional <minor />
        // optional <comment>...</comment> or <comment deleted="deleted" />
        // optional <origin>...</origin>

        let mut next_byte = self.stream.read_byte()?;

        if ![b'm', b'c', b'o'].contains(&next_byte) {
            return Err(Box::new(QwiktError(
                format!("Expected 'm', 'c' or 'o', got {:?}", next_byte as char)
            )));
        }

        if next_byte == b'm' {
            self.stream.match_exact(b"inor />\n      <")?;
            next_byte = self.stream.read_byte()?;
        }

        if next_byte == b'c' {
            self.stream.match_exact(b"omment")?;
            let byte = self.stream.read_byte()?;
            if byte == b'>' {
                let _comment = self.stream.read_until(b'<')?;
                self.stream.match_exact(b"/comment>\n      <")?;
                self.stream.read_byte()?;
            } else if byte == b' ' {
                self.stream.match_exact(b"deleted=\"deleted\" />\n      <")?;
                self.stream.read_byte()?;
            }
        }

        self.stream.match_exact(b"rigin>")?;
        let _origin = self.stream.read_until(b'<')?;
        self.stream.match_exact(b"/origin>\n      <")?;

        self.stream.match_exact(b"model>")?;
        let _model = self.stream.read_until(b'<')?;

        self.stream.match_exact(b"/model>\n      <")?;

        self.stream.match_exact(b"format>")?;
        let _format = self.stream.read_until(b'<')?;

        self.stream.match_exact(b"/format>\n      <text bytes=\"")?;
        let _text_bytes = self.stream.read_until(b'"')?;

        self.stream.match_exact(b" sha1=\"")?;
        let _text_sha1 = self.stream.read_until(b'"')?;

        self.stream.match_exact(b" ")?;
        let byte = self.stream.read_byte()?;
        let rev_text = if byte == b'/' {
            self.stream.match_exact(b">\n      <sha1>")?;
            String::new()
        } else if byte == b'x' {
            self.stream.match_exact(b"ml:space=\"preserve\">")?;
            let text_body = self.stream.read_until(b'<')?;
            self.stream.match_exact(b"/text>\n      <sha1>")?;
            String::from_utf8_lossy(&text_body).into_owned()
        } else {
            return Err(Box::new(QwiktError(format!("Expected '/' or 'x', got {:?}", byte as char))));
        };

        let _sha1_b = self.stream.read_until(b'<')?;
        self.stream.match_exact(b"/sha1>\n    </revision>\n  </page>\n")?;

        Ok(Some(Page {
            title,
            ns,
            id,
            rev_id: None, // Not tracked in output
            rev_contrib_id,
            rev_text,
        }))
    }
}
