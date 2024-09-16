use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unexpected EOF")]
    UnexpectedEOF,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub struct Cursor<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    pub fn read(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if self.position + n > self.input.len() {
            return Err(Error::UnexpectedEOF);
        }
        let slice = &self.input[self.position..self.position + n];
        self.position += n;
        Ok(slice)
    }

    pub fn read_byte(&mut self) -> Result<u8, Error> {
        if self.position >= self.input.len() {
            return Err(Error::UnexpectedEOF);
        }
        let byte = self.input[self.position];
        self.position += 1;
        Ok(byte)
    }

    pub fn read_line(&mut self) -> Result<&'a [u8], Error> {
        let start = self.position;
        while self.position < self.input.len() - 1 {
            if self.input[self.position] == b'\r' && self.input[self.position + 1] == b'\n' {
                let line = &self.input[start..self.position];
                self.position += 2;
                return Ok(line);
            }
            self.position += 1;
        }
        Err(Error::UnexpectedEOF)
    }

    pub fn read_string(&mut self) -> Result<String, Error> {
        let line = self.read_line()?;
        String::from_utf8(line.to_vec())
            .map_err(|_| Error::InvalidInput(format!("'{:?}' is not a valid UTF-8 sequence", line)))
    }

    pub fn read_integer(&mut self) -> Result<i64, Error> {
        let line = self.read_line()?;
        let integer = std::str::from_utf8(line)
            .map_err(|_| Error::InvalidInput(format!("'{:?}' is not a valid UTF-8 sequence", line)))
            .and_then(|s| {
                s.parse::<i64>().map_err(|_| {
                    Error::InvalidInput(format!("'{:?}' is not a valid integer", line))
                })
            })?;
        Ok(integer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_new() {
        let input = b"hello";
        let cursor = Cursor::new(input);
        assert_eq!(cursor.position, 0);
        assert_eq!(cursor.input, input);
    }

    #[test]
    fn read_byte_success() {
        let input = b"ab";
        let mut cursor = Cursor::new(input);

        assert_eq!(cursor.read_byte().unwrap(), b'a');
        assert_eq!(cursor.position, 1);

        assert_eq!(cursor.read_byte().unwrap(), b'b');
        assert_eq!(cursor.position, 2);
    }

    #[test]
    fn read_byte_eof() {
        let input = b"a";
        let mut cursor = Cursor::new(input);

        assert_eq!(cursor.read_byte().unwrap(), b'a');
        assert_eq!(cursor.position, 1);

        assert!(matches!(cursor.read_byte(), Err(Error::UnexpectedEOF)));
        assert_eq!(cursor.position, 1);
    }

    #[test]
    fn read_line_success() {
        let input = b"hello\r\nworld\r\n";
        let mut cursor = Cursor::new(input);

        assert_eq!(cursor.read_line().unwrap(), b"hello");
        assert_eq!(cursor.position, 7);

        assert_eq!(cursor.read_line().unwrap(), b"world");
        assert_eq!(cursor.position, 14);
    }

    #[test]
    fn read_line_no_crlf() {
        let input = b"hello";
        let mut cursor = Cursor::new(input);

        assert!(matches!(cursor.read_line(), Err(Error::UnexpectedEOF)));
        assert_eq!(cursor.position, 4);
    }

    #[test]
    fn read_line_empty() {
        let input = b"\r\n";
        let mut cursor = Cursor::new(input);

        assert_eq!(cursor.read_line().unwrap(), b"");
        assert_eq!(cursor.position, 2);
    }

    #[test]
    fn read_string_success() {
        let input = "hello\r\nworld\r\n".as_bytes();
        let mut cursor = Cursor::new(input);

        assert_eq!(cursor.read_string().unwrap(), "hello");
        assert_eq!(cursor.position, 7);

        assert_eq!(cursor.read_string().unwrap(), "world");
        assert_eq!(cursor.position, 14);
    }

    #[test]
    fn read_string_empty() {
        let input = "\r\n".as_bytes();
        let mut cursor = Cursor::new(input);

        assert_eq!(cursor.read_string().unwrap(), "");
        assert_eq!(cursor.position, 2);
    }

    #[test]
    fn read_string_invalid_utf8() {
        let input = &[0xFF, b'\r', b'\n'];
        let mut cursor = Cursor::new(input);

        match cursor.read_string() {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("is not a valid UTF-8 sequence"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
        assert_eq!(cursor.position, 3);
    }

    #[test]
    fn read_string_eof() {
        let input = "hello".as_bytes();
        let mut cursor = Cursor::new(input);

        assert!(matches!(cursor.read_string(), Err(Error::UnexpectedEOF)));
        assert_eq!(cursor.position, 4);
    }

    #[test]
    fn read_integer_success() {
        let input = "42\r\n-123\r\n0\r\n".as_bytes();
        let mut cursor = Cursor::new(input);

        assert_eq!(cursor.read_integer().unwrap(), 42);
        assert_eq!(cursor.position, 4);

        assert_eq!(cursor.read_integer().unwrap(), -123);
        assert_eq!(cursor.position, 10);

        assert_eq!(cursor.read_integer().unwrap(), 0);
        assert_eq!(cursor.position, 13);
    }

    #[test]
    fn read_integer_invalid_input() {
        let input = "not_a_number\r\n".as_bytes();
        let mut cursor = Cursor::new(input);

        match cursor.read_integer() {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("is not a valid integer"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
        assert_eq!(cursor.position, 14);
    }

    #[test]
    fn read_integer_empty() {
        let input = "\r\n".as_bytes();
        let mut cursor = Cursor::new(input);

        match cursor.read_integer() {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("is not a valid integer"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
        assert_eq!(cursor.position, 2);
    }

    #[test]
    fn read_integer_out_of_range() {
        let input = "9223372036854775808\r\n".as_bytes();
        let mut cursor = Cursor::new(input);

        match cursor.read_integer() {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("is not a valid integer"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
        assert_eq!(cursor.position, 21);
    }
}
