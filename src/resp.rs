use thiserror::Error;

/// The possible values of a RESP protocol payload
#[derive(Debug, PartialEq, Eq)]
enum RespValue {
    SimpleString(String),
}

/// Parse the binary input as a sequence of ASCII characters encoded in resp2 protocol
fn parse(input: &[u8]) -> Result<RespValue, Error> {
    let first_char = input.first().ok_or(Error::UnexpectedEOF)?;
    match first_char {
        b'+' => parse_simple_string(&input[1..]),
        _ => Err(Error::UnexpectedToken),
    }
}

/// Parse a simple string consumes input until \r\n
fn parse_simple_string(input: &[u8]) -> Result<RespValue, Error> {
    let mut string = String::new();
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'\r' && input[i + 1] == b'\n' {
            return Ok(RespValue::SimpleString(string));
        }
        string.push(input[i] as char);
        i += 1;
    }
    Err(Error::UnexpectedEOF)
}

#[derive(Error, Debug, PartialEq, Eq)]
enum Error {
    #[error("unexpected EOF")]
    UnexpectedEOF,
    #[error("unexpected token")]
    UnexpectedToken,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let input = b"";
        let parsed = parse(input);
        assert_eq!(parsed, Err(Error::UnexpectedEOF));
    }

    #[test]
    fn test_parse_unexpected_token() {
        let input = b"?";
        let parsed = parse(input);
        assert_eq!(parsed, Err(Error::UnexpectedToken));
    }

    #[test]
    fn test_parse_simple_string() {
        let input = b"+OK\r\n";
        let parsed = parse(input).unwrap();
        assert_eq!(parsed, RespValue::SimpleString("OK".to_string()));
    }

    #[test]
    fn test_parse_simple_string_with_newline() {
        let input = b"+OK\n";
        let parsed = parse(input);
        assert_eq!(parsed, Err(Error::UnexpectedEOF));
    }
}
