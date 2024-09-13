use thiserror::Error;

/// The possible values of a RESP protocol payload
#[derive(Debug, PartialEq, Eq)]
pub enum RespValue {
    SimpleString(String),
    Error(String),
}

/// Parse the binary input as a sequence of ASCII characters encoded in resp2 protocol
pub fn parse(input: &[u8]) -> Result<RespValue, Error> {
    let first_char = input.first().ok_or(Error::UnexpectedEOF)?;
    match first_char {
        b'+' => parse_simple_string(&input[1..]),
        b'-' => parse_error(&input[1..]),
        _ => Err(Error::UnexpectedToken),
    }
}

/// Parse a simple string consuming the input until \r\n
fn parse_simple_string(input: &[u8]) -> Result<RespValue, Error> {
    parse_simple_bitstring(input).map(RespValue::SimpleString)
}

/// Parse an error consuming the input until \r\n
fn parse_error(input: &[u8]) -> Result<RespValue, Error> {
    parse_simple_bitstring(input).map(RespValue::Error)
}

fn parse_simple_bitstring(input: &[u8]) -> Result<String, Error> {
    if input.is_empty() {
        return Err(Error::UnexpectedEOF);
    }

    if input[input.len() - 2..] != [b'\r', b'\n'] {
        return Err(Error::UnexpectedEOF);
    }

    let mut string = String::new();
    let mut i = 0;
    while i < input.len() - 2 {
        string.push(input[i] as char);
        i += 1;
    }

    Ok(string)
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
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
