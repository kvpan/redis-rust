use thiserror::Error;

struct Cursor<'a> {
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

pub enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(String),
    Null,
    Array(Vec<RespValue>),
    True,
    False,
    Double(f64),
    PositiveInfinity,
    NegativeInfinity,
    NaN,
    BigNumber(String),
    BulkError(String),
}

pub fn parse(input: &[u8]) -> Result<RespValue, Error> {
    let mut cursor = Cursor::new(input);
    parse_value(&mut cursor)
}

fn parse_value(cursor: &mut Cursor) -> Result<RespValue, Error> {
    let first_byte = cursor.read_byte()? as char;
    match first_byte {
        '+' => {
            let string = cursor.read_string()?;
            Ok(RespValue::SimpleString(string))
        }
        '-' => {
            let string = cursor.read_string()?;
            Ok(RespValue::Error(string))
        }
        ':' => {
            let integer = cursor.read_integer()?;
            Ok(RespValue::Integer(integer))
        }
        '$' => {
            // TODO: Handle the case where the length is too large
            let len = cursor.read_integer()?;

            if len == -1 {
                return Ok(RespValue::Null);
            }

            let data = cursor.read(len as usize)?;
            let string = std::str::from_utf8(data).map_err(|_| {
                Error::InvalidInput(format!("'{:?}' is not a valid UTF-8 sequence", data))
            })?;

            let terminator = cursor.read(2)?;
            if terminator != b"\r\n" {
                return Err(Error::InvalidInput(format!(
                    "unexpected bytes after bulk string: {:?}",
                    terminator
                )));
            }

            Ok(RespValue::BulkString(string.to_string()))
        }
        '*' => {
            let len = cursor.read_integer()?;

            if len == -1 {
                return Ok(RespValue::Null);
            }

            let items = (0..len)
                .map(|_| parse_value(cursor))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(RespValue::Array(items))
        }
        '_' => {
            let terminator = cursor.read(2)?;
            if terminator != b"\r\n" {
                return Err(Error::InvalidInput(format!(
                    "unexpected bytes after null: {:?}",
                    terminator
                )));
            }
            Ok(RespValue::Null)
        }
        '#' => {
            let value = cursor.read_byte()?;
            match value {
                b't' => Ok(RespValue::True),
                b'f' => Ok(RespValue::False),
                _ => Err(Error::InvalidInput(format!(
                    "unexpected byte after #: {}",
                    value
                ))),
            }
        }
        ',' => {
            let value = cursor.read_line()?;
            match value {
                b"inf" => Ok(RespValue::PositiveInfinity),
                b"-inf" => Ok(RespValue::NegativeInfinity),
                b"nan" => Ok(RespValue::NaN),
                _ => {
                    let double = std::str::from_utf8(value)
                        .map_err(|_| Error::InvalidInput(format!("invalid double: {:?}", value)))
                        .and_then(|s| {
                            s.parse::<f64>().map_err(|_| {
                                Error::InvalidInput(format!("invalid double: {:?}", value))
                            })
                        })?;
                    Ok(RespValue::Double(double))
                }
            }
        }
        '(' => {
            let value = cursor.read_string()?;

            if !value.starts_with(|c: char| c == '+' || c == '-') {
                return Err(Error::InvalidInput(format!(
                    "invalid big number: {:?}",
                    value
                )));
            }

            for c in value.chars().skip(1) {
                if !c.is_digit(10) {
                    return Err(Error::InvalidInput(format!(
                        "invalid big number: {:?}",
                        value
                    )));
                }
            }

            Ok(RespValue::BigNumber(value))
        }
        '!' => {
            let len = cursor.read_integer()?;
            let data = cursor.read(len as usize)?;
            let string = std::str::from_utf8(data).map_err(|_| {
                Error::InvalidInput(format!("'{:?}' is not a valid UTF-8 sequence", data))
            })?;
            Ok(RespValue::BulkError(string.to_string()))
        }
        _ => Err(Error::InvalidInput(format!(
            "unexpected first byte: {}",
            first_byte
        ))),
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("unexpected EOF")]
    UnexpectedEOF,
    #[error("invalid input: {0}")]
    InvalidInput(String),
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

    #[test]
    fn parse_simple_string() {
        let input = b"+hello\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::SimpleString(s) if s == "hello"));
    }

    #[test]
    fn parse_empty_simple_string() {
        let input = b"+\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::SimpleString(s) if s.is_empty()));
    }

    #[test]
    fn parse_invalid_first_byte() {
        let input = b"/hello\r\n";
        match parse(input) {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("unexpected first byte"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn parse_incomplete_input() {
        let input = b"+hello";
        assert!(matches!(parse(input), Err(Error::UnexpectedEOF)));
    }

    #[test]
    fn parse_error() {
        let input = b"-Error message\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Error(s) if s == "Error message"));
    }

    #[test]
    fn parse_empty_error() {
        let input = b"-\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Error(s) if s.is_empty()));
    }

    #[test]
    fn parse_integer_positive() {
        let input = b":42\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Integer(n) if n == 42));
    }

    #[test]
    fn parse_integer_negative() {
        let input = b":-123\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Integer(n) if n == -123));
    }

    #[test]
    fn parse_integer_zero() {
        let input = b":0\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Integer(n) if n == 0));
    }

    #[test]
    fn parse_integer_max() {
        let input = b":9223372036854775807\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Integer(n) if n == i64::MAX));
    }

    #[test]
    fn parse_integer_min() {
        let input = b":-9223372036854775808\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Integer(n) if n == i64::MIN));
    }

    #[test]
    fn parse_integer_invalid() {
        let input = b":not_a_number\r\n";
        match parse(input) {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("is not a valid integer"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn parse_bulk_string() {
        let input = b"$5\r\nhello\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::BulkString(s) if s == "hello"));
    }

    #[test]
    fn parse_empty_bulk_string() {
        let input = b"$0\r\n\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::BulkString(s) if s.is_empty()));
    }

    #[test]
    fn parse_null_bulk_string() {
        let input = b"$-1\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Null));
    }

    #[test]
    fn parse_bulk_string_with_special_chars() {
        let input = b"$8\r\nfoo\r\nbar\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::BulkString(s) if s == "foo\r\nbar"));
    }

    #[test]
    fn parse_bulk_string_length_mismatch() {
        let input = b"$10\r\nhello\r\n";
        assert!(matches!(parse(input), Err(Error::UnexpectedEOF)));
    }

    #[test]
    fn parse_empty_array() {
        let input = b"*0\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Array(arr) if arr.is_empty()));
    }

    #[test]
    fn parse_simple_array() {
        let input = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let result = parse(input).unwrap();
        if let RespValue::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert!(matches!(&arr[0], RespValue::BulkString(s) if s == "hello"));
            assert!(matches!(&arr[1], RespValue::BulkString(s) if s == "world"));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn parse_nested_array() {
        let input = b"*3\r\n:1\r\n*2\r\n+Hello\r\n-Error\r\n$5\r\nworld\r\n";
        let result = parse(input).unwrap();
        if let RespValue::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert!(matches!(&arr[0], RespValue::Integer(n) if *n == 1));
            if let RespValue::Array(nested) = &arr[1] {
                assert_eq!(nested.len(), 2);
                assert!(matches!(&nested[0], RespValue::SimpleString(s) if s == "Hello"));
                assert!(matches!(&nested[1], RespValue::Error(s) if s == "Error"));
            } else {
                panic!("Expected nested Array");
            }
            assert!(matches!(&arr[2], RespValue::BulkString(s) if s == "world"));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn parse_null_array() {
        let input = b"*-1\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Null));
    }

    #[test]
    fn parse_incomplete_array() {
        let input = b"*2\r\n:1\r\n";
        assert!(matches!(parse(input), Err(Error::UnexpectedEOF)));
    }

    #[test]
    fn parse_null_value() {
        let input = b"_\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Null));
    }

    #[test]
    fn parse_true_value() {
        let input = b"#t\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::True));
    }

    #[test]
    fn parse_false_value() {
        let input = b"#f\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::False));
    }

    #[test]
    fn parse_double_valid() {
        let input = b",123.45\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Double(n) if n == 123.45));
    }

    #[test]
    fn parse_double_negative() {
        let input = b",-123.45\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Double(n) if n == -123.45));
    }

    #[test]
    fn parse_double_nan() {
        let input = b",nan\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::NaN));
    }

    #[test]
    fn parse_double_infinity() {
        let input = b",inf\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::PositiveInfinity));
    }

    #[test]
    fn parse_double_negative_infinity() {
        let input = b",-inf\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::NegativeInfinity));
    }

    #[test]
    fn parse_double_invalid() {
        let input = b",not_a_number\r\n";
        match parse(input) {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("invalid double"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn parse_big_number_valid() {
        let input = b"(+12345678901234567890\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::BigNumber(s) if s == "+12345678901234567890"));
    }

    #[test]
    fn parse_big_number_negative() {
        let input = b"(-12345678901234567890\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::BigNumber(s) if s == "-12345678901234567890"));
    }

    #[test]
    fn parse_big_number_invalid_chars() {
        let input = b"(123abc\r\n";
        match parse(input) {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("invalid big number"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn parse_big_number_missing_sign() {
        let input = b"(12345678901234567890\r\n";
        match parse(input) {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("invalid big number"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn parse_bulk_error() {
        let input = b"!13\r\nerror message\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::BulkError(s) if s == "error message"));
    }

    #[test]
    fn parse_empty_bulk_error() {
        let input = b"!0\r\n\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::BulkError(s) if s.is_empty()));
    }
}
