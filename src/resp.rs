use crate::{cursor::Cursor, cursor::Error};

#[derive(Debug)]
pub enum RespValue {
    Array(Vec<RespValue>),
    BigNumber(String),
    BulkError(String),
    BulkString(String),
    Double(f64),
    Error(String),
    False,
    Integer(i64),
    Map(Vec<(RespValue, RespValue)>),
    NaN,
    NegativeInfinity,
    Null,
    NullBulkString,
    PositiveInfinity,
    Set(Vec<RespValue>),
    SimpleString(String),
    True,
    VerbatimString(String, String),
}

impl RespValue {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            RespValue::Array(values) => {
                let mut array = Vec::new();
                array.push(b'*');
                array.extend_from_slice(values.len().to_string().as_bytes());
                array.extend_from_slice(b"\r\n");

                for value in values {
                    array.extend(value.as_bytes());
                }

                array
            }
            RespValue::BigNumber(string) => {
                let mut array = Vec::new();
                array.push(b'(');
                array.extend_from_slice(string.as_bytes());
                array.extend_from_slice(b"\r\n");
                array
            }
            RespValue::BulkError(string) => {
                let mut array = Vec::new();
                array.push(b'!');
                array.extend_from_slice(string.len().to_string().as_bytes());
                array.extend_from_slice(b"\r\n");
                array.extend_from_slice(string.as_bytes());
                array.extend_from_slice(b"\r\n");
                array
            }
            RespValue::BulkString(string) => {
                let mut array = Vec::new();
                array.push(b'$');
                array.extend_from_slice(string.len().to_string().as_bytes());
                array.extend_from_slice(b"\r\n");
                array.extend_from_slice(string.as_bytes());
                array.extend_from_slice(b"\r\n");
                array
            }
            RespValue::Double(value) => {
                let mut array = Vec::new();
                array.push(b',');
                array.extend_from_slice(value.to_string().as_bytes());
                array.extend_from_slice(b"\r\n");
                array
            }
            RespValue::Error(string) => {
                let mut array = Vec::new();
                array.push(b'-');
                array.extend_from_slice(string.as_bytes());
                array.extend_from_slice(b"\r\n");
                array
            }
            RespValue::False => {
                let mut array = Vec::new();
                array.extend_from_slice(b"#f\r\n");
                array
            }
            RespValue::Integer(value) => {
                let mut array = Vec::new();
                array.push(b':');
                array.extend_from_slice(value.to_string().as_bytes());
                array.extend_from_slice(b"\r\n");
                array
            }
            RespValue::Map(entries) => {
                let mut array = Vec::new();
                array.push(b'%');
                array.extend_from_slice(entries.len().to_string().as_bytes());
                array.extend_from_slice(b"\r\n");

                for (key, value) in entries {
                    array.extend(key.as_bytes());
                    array.extend(value.as_bytes());
                }

                array
            }
            RespValue::NaN => {
                let mut array = Vec::new();
                array.extend_from_slice(b",nan\r\n");
                array
            }
            RespValue::NegativeInfinity => {
                let mut array = Vec::new();
                array.extend_from_slice(b",-inf\r\n");
                array
            }
            RespValue::Null => {
                let mut array = Vec::new();
                array.extend_from_slice(b"_\r\n");
                array
            }
            RespValue::PositiveInfinity => {
                let mut array = Vec::new();
                array.extend_from_slice(b",inf\r\n");
                array
            }
            RespValue::Set(values) => {
                let mut array = Vec::new();
                array.push(b'~');
                array.extend_from_slice(values.len().to_string().as_bytes());
                array.extend_from_slice(b"\r\n");

                for value in values {
                    array.extend(value.as_bytes());
                }

                array
            }
            RespValue::SimpleString(string) => {
                let mut array = Vec::new();
                array.push(b'+');
                array.extend_from_slice(string.as_bytes());
                array.extend_from_slice(b"\r\n");
                array
            }
            RespValue::True => {
                let mut array = Vec::new();
                array.extend_from_slice(b"#t\r\n");
                array
            }
            RespValue::VerbatimString(encoding, string) => {
                let len = encoding.len() + string.len() + 1;
                let mut array = Vec::new();
                array.push(b'=');
                array.extend_from_slice(len.to_string().as_bytes());
                array.extend_from_slice(b"\r\n");
                array.extend_from_slice(encoding.as_bytes());
                array.push(b':');
                array.extend_from_slice(string.as_bytes());
                array.extend_from_slice(b"\r\n");
                array
            }
            RespValue::NullBulkString => {
                let mut array = Vec::new();
                array.extend_from_slice(b"$-1\r\n");
                array
            }
        }
    }
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
                return Ok(RespValue::NullBulkString);
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
        '=' => {
            let len = cursor.read_integer()?;
            let data = cursor.read(len as usize)?;

            if data[3] != b':' {
                return Err(Error::InvalidInput(format!(
                    "invalid verbatim string: {:?}",
                    data
                )));
            }

            let encoding = std::str::from_utf8(&data[..3])
                .map_err(|_| Error::InvalidInput(format!("invalid encoding: {:?}", data)))?;

            let string = std::str::from_utf8(&data[4..]).map_err(|_| {
                Error::InvalidInput(format!("'{:?}' is not a valid UTF-8 sequence", data))
            })?;

            Ok(RespValue::VerbatimString(
                encoding.to_string(),
                string.to_string(),
            ))
        }
        '%' => {
            let len = cursor.read_integer()?;
            let mut entries = Vec::new();

            for _ in 0..len {
                let key = parse_value(cursor)?;
                let value = parse_value(cursor)?;
                entries.push((key, value));
            }

            Ok(RespValue::Map(entries))
        }
        '~' => {
            let len = cursor.read_integer()?;
            let mut entries = Vec::new();

            for _ in 0..len {
                let value = parse_value(cursor)?;
                entries.push(value);
            }

            Ok(RespValue::Set(entries))
        }
        _ => Err(Error::InvalidInput(format!(
            "unexpected first byte: {}",
            first_byte
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(matches!(result, RespValue::NullBulkString));
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

    #[test]
    fn parse_verbatim_string() {
        let input = b"=15\r\ntxt:Some string\r\n";
        let result = parse(input).unwrap();
        assert!(
            matches!(result, RespValue::VerbatimString(enc, s) if enc == "txt" && s == "Some string")
        );
    }

    #[test]
    fn parse_verbatim_string_length_mismatch() {
        let input = b"=20\r\ntxt:Some string\r\n";
        assert!(matches!(parse(input), Err(Error::UnexpectedEOF)));
    }

    #[test]
    fn parse_invalid_verbatim_string_encoding() {
        let input = b"=11\r\nSome string\r\n";
        match parse(input) {
            Err(Error::InvalidInput(msg)) => {
                assert!(msg.contains("invalid verbatim string"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn parse_map() {
        let input = b"%2\r\n$3\r\nkey\r\n$5\r\nvalue\r\n+OK\r\n:2\r\n";
        let result = parse(input).unwrap();
        if let RespValue::Map(map) = result {
            assert_eq!(map.len(), 2);
            assert!(
                matches!(&map[0], (RespValue::BulkString(k), RespValue::BulkString(v)) if k == "key" && v == "value")
            );
            assert!(
                matches!(&map[1], (RespValue::SimpleString(k), RespValue::Integer(v)) if k == "OK" && *v == 2)
            );
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn parse_empty_map() {
        let input = b"%0\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Map(map) if map.is_empty()));
    }

    #[test]
    fn parse_map_with_null_values() {
        let input = b"%1\r\n$3\r\nkey\r\n_\r\n";
        let result = parse(input).unwrap();
        if let RespValue::Map(map) = result {
            assert_eq!(map.len(), 1);
            assert!(matches!(&map[0], (RespValue::BulkString(k), RespValue::Null) if k == "key"));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn parse_incomplete_map() {
        let input = b"%2\r\n$3\r\nkey\r\n";
        assert!(matches!(parse(input), Err(Error::UnexpectedEOF)));
    }

    #[test]
    fn parse_empty_set() {
        let input = b"~0\r\n";
        let result = parse(input).unwrap();
        assert!(matches!(result, RespValue::Set(set) if set.is_empty()));
    }

    #[test]
    fn parse_simple_set() {
        let input = b"~2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let result = parse(input).unwrap();
        if let RespValue::Set(set) = result {
            assert_eq!(set.len(), 2);
            assert!(matches!(&set[0], RespValue::BulkString(s) if s == "hello"));
            assert!(matches!(&set[1], RespValue::BulkString(s) if s == "world"));
        } else {
            panic!("Expected Set");
        }
    }

    #[test]
    fn parse_incomplete_set() {
        let input = b"~2\r\n$5\r\nhello\r\n";
        assert!(matches!(parse(input), Err(Error::UnexpectedEOF)));
    }

    #[test]
    fn array_as_bytes_empty() {
        let array = RespValue::Array(vec![]);
        let result = array.as_bytes();
        assert_eq!(result, b"*0\r\n");
    }

    #[test]
    fn array_as_bytes_single_element() {
        let array = RespValue::Array(vec![RespValue::SimpleString("hello".to_string())]);
        let result = array.as_bytes();
        assert_eq!(result, b"*1\r\n+hello\r\n");
    }

    #[test]
    fn array_as_bytes_multiple_elements() {
        let array = RespValue::Array(vec![
            RespValue::SimpleString("hello".to_string()),
            RespValue::SimpleString("world".to_string()),
        ]);
        let result = array.as_bytes();
        assert_eq!(result, b"*2\r\n+hello\r\n+world\r\n");
    }

    #[test]
    fn simple_string_as_bytes() {
        let input = RespValue::SimpleString("hello".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"+hello\r\n");
    }

    #[test]
    fn empty_simple_string_as_bytes() {
        let input = RespValue::SimpleString("".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"+\r\n");
    }

    #[test]
    fn big_number_as_bytes() {
        let big_number = RespValue::BigNumber("+12345678901234567890".to_string());
        let result = big_number.as_bytes();
        assert_eq!(result, b"(+12345678901234567890\r\n");
    }

    #[test]
    fn big_number_negative_as_bytes() {
        let big_number = RespValue::BigNumber("-12345678901234567890".to_string());
        let result = big_number.as_bytes();
        assert_eq!(result, b"(-12345678901234567890\r\n");
    }

    #[test]
    fn bulk_error_as_bytes() {
        let input = RespValue::BulkError("error message".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"!13\r\nerror message\r\n");
    }

    #[test]
    fn empty_bulk_error_as_bytes() {
        let input = RespValue::BulkError("".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"!0\r\n\r\n");
    }

    #[test]
    fn bulk_string_as_bytes() {
        let input = RespValue::BulkString("hello".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"$5\r\nhello\r\n");
    }

    #[test]
    fn empty_bulk_string_as_bytes() {
        let input = RespValue::BulkString("".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"$0\r\n\r\n");
    }

    #[test]
    fn bulk_string_with_special_chars_as_bytes() {
        let input = RespValue::BulkString("foo\r\nbar".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"$8\r\nfoo\r\nbar\r\n");
    }

    #[test]
    fn double_as_bytes_valid() {
        let input = RespValue::Double(123.45);
        let result = input.as_bytes();
        assert_eq!(result, b",123.45\r\n");
    }

    #[test]
    fn double_as_bytes_negative() {
        let input = RespValue::Double(-123.45);
        let result = input.as_bytes();
        assert_eq!(result, b",-123.45\r\n");
    }

    #[test]
    fn error_as_bytes() {
        let input = RespValue::Error("Error message".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"-Error message\r\n");
    }

    #[test]
    fn empty_error_as_bytes() {
        let input = RespValue::Error("".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"-\r\n");
    }

    #[test]
    fn false_as_bytes() {
        let input = RespValue::False;
        let result = input.as_bytes();
        assert_eq!(result, b"#f\r\n");
    }

    #[test]
    fn integer_as_bytes_positive() {
        let input = RespValue::Integer(42);
        let result = input.as_bytes();
        assert_eq!(result, b":42\r\n");
    }

    #[test]
    fn integer_as_bytes_negative() {
        let input = RespValue::Integer(-123);
        let result = input.as_bytes();
        assert_eq!(result, b":-123\r\n");
    }

    #[test]
    fn integer_as_bytes_zero() {
        let input = RespValue::Integer(0);
        let result = input.as_bytes();
        assert_eq!(result, b":0\r\n");
    }

    #[test]
    fn map_as_bytes_empty() {
        let map = RespValue::Map(vec![]);
        let result = map.as_bytes();
        assert_eq!(result, b"%0\r\n");
    }

    #[test]
    fn map_as_bytes_single_entry() {
        let map = RespValue::Map(vec![(
            RespValue::BulkString("key".to_string()),
            RespValue::BulkString("value".to_string()),
        )]);
        let result = map.as_bytes();
        assert_eq!(result, b"%1\r\n$3\r\nkey\r\n$5\r\nvalue\r\n");
    }

    #[test]
    fn map_as_bytes_multiple_entries() {
        let map = RespValue::Map(vec![
            (
                RespValue::BulkString("key1".to_string()),
                RespValue::BulkString("value1".to_string()),
            ),
            (
                RespValue::SimpleString("key2".to_string()),
                RespValue::Integer(42),
            ),
        ]);
        let result = map.as_bytes();
        assert_eq!(
            result,
            b"%2\r\n$4\r\nkey1\r\n$6\r\nvalue1\r\n+key2\r\n:42\r\n"
        );
    }

    #[test]
    fn double_as_bytes_nan() {
        let input = RespValue::NaN;
        let result = input.as_bytes();
        assert_eq!(result, b",nan\r\n");
    }

    #[test]
    fn double_as_bytes_negative_infinity() {
        let input = RespValue::NegativeInfinity;
        let result = input.as_bytes();
        assert_eq!(result, b",-inf\r\n");
    }

    #[test]
    fn null_as_bytes() {
        let input = RespValue::Null;
        let result = input.as_bytes();
        assert_eq!(result, b"_\r\n");
    }

    #[test]
    fn double_as_bytes_positive_infinity() {
        let input = RespValue::PositiveInfinity;
        let result = input.as_bytes();
        assert_eq!(result, b",inf\r\n");
    }

    #[test]
    fn set_as_bytes_empty() {
        let set = RespValue::Set(vec![]);
        let result = set.as_bytes();
        assert_eq!(result, b"~0\r\n");
    }

    #[test]
    fn set_as_bytes_single_element() {
        let set = RespValue::Set(vec![RespValue::SimpleString("hello".to_string())]);
        let result = set.as_bytes();
        assert_eq!(result, b"~1\r\n+hello\r\n");
    }

    #[test]
    fn set_as_bytes_multiple_elements() {
        let set = RespValue::Set(vec![
            RespValue::SimpleString("hello".to_string()),
            RespValue::SimpleString("world".to_string()),
        ]);
        let result = set.as_bytes();
        assert_eq!(result, b"~2\r\n+hello\r\n+world\r\n");
    }

    #[test]
    fn true_as_bytes() {
        let input = RespValue::True;
        let result = input.as_bytes();
        assert_eq!(result, b"#t\r\n");
    }

    #[test]
    fn verbatim_string_as_bytes() {
        let input = RespValue::VerbatimString("txt".to_string(), "Some string".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"=15\r\ntxt:Some string\r\n");
    }

    #[test]
    fn empty_verbatim_string_as_bytes() {
        let input = RespValue::VerbatimString("txt".to_string(), "".to_string());
        let result = input.as_bytes();
        assert_eq!(result, b"=4\r\ntxt:\r\n");
    }
}
