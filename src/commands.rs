use crate::cursor::Error as RespError;
use crate::resp::parse;
use crate::resp::RespValue;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    ConfigGet(String),
    Echo(String),
    Get(String),
    Ping,
    Set(String, String, Option<u64>),
}

impl Command {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RespError> {
        let RespValue::Array(arr) = parse(bytes)? else {
            return Err(RespError::InvalidInput(
                String::from_utf8(bytes.to_vec()).unwrap(),
            ));
        };

        if arr.len() == 0 {
            return Err(RespError::InvalidInput(
                String::from_utf8(bytes.to_vec()).unwrap(),
            ));
        }

        let RespValue::BulkString(cmd) = arr.get(0).unwrap() else {
            return Err(RespError::InvalidInput(
                String::from_utf8(bytes.to_vec()).unwrap(),
            ));
        };

        match cmd.to_ascii_uppercase().as_str() {
            "CONFIG" => {
                if arr.len() < 3 {
                    return Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    ));
                }

                let RespValue::BulkString(subcmd) = arr.get(1).unwrap() else {
                    return Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    ));
                };

                match subcmd.to_ascii_uppercase().as_str() {
                    "GET" => {
                        if arr.len() != 3 {
                            return Err(RespError::InvalidInput(
                                String::from_utf8(bytes.to_vec()).unwrap(),
                            ));
                        }

                        let RespValue::BulkString(key) = arr.get(2).unwrap() else {
                            return Err(RespError::InvalidInput(
                                String::from_utf8(bytes.to_vec()).unwrap(),
                            ));
                        };

                        Ok(Command::ConfigGet(key.to_string()))
                    }
                    _ => Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    )),
                }
            }
            "ECHO" => {
                if arr.len() != 2 {
                    return Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    ));
                }

                let RespValue::BulkString(msg) = arr.get(1).unwrap() else {
                    return Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    ));
                };

                Ok(Command::Echo(msg.to_string()))
            }
            "GET" => {
                if arr.len() != 2 {
                    return Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    ));
                }

                let RespValue::BulkString(key) = arr.get(1).unwrap() else {
                    return Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    ));
                };

                Ok(Command::Get(key.to_string()))
            }
            "PING" => Ok(Command::Ping),
            "SET" => {
                if arr.len() < 3 {
                    return Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    ));
                }

                let RespValue::BulkString(key) = arr.get(1).unwrap() else {
                    return Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    ));
                };

                let RespValue::BulkString(value) = arr.get(2).unwrap() else {
                    return Err(RespError::InvalidInput(
                        String::from_utf8(bytes.to_vec()).unwrap(),
                    ));
                };

                let mut expiry = None;

                for chunk in arr[3..].chunks(2) {
                    if let [RespValue::BulkString(opt), RespValue::BulkString(val)] = chunk {
                        if opt.to_ascii_uppercase() == "PX" {
                            expiry = Some(val.parse::<u64>().unwrap());
                        }
                    }
                }

                Ok(Command::Set(key.to_string(), value.to_string(), expiry))
            }
            _ => Err(RespError::InvalidInput(
                String::from_utf8(bytes.to_vec()).unwrap(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_command() {
        let input = b"*1\r\n$4\r\nPING\r\n";
        let command = Command::from_bytes(input).unwrap();
        assert_eq!(command, Command::Ping);
    }

    #[test]
    fn test_echo_command() {
        let input = b"*2\r\n$4\r\nECHO\r\n$5\r\nHello\r\n";
        let command = Command::from_bytes(input).unwrap();
        assert_eq!(command, Command::Echo("Hello".to_string()));
    }

    #[test]
    fn test_set_command_with_expiry() {
        let input = b"*5\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n$2\r\nPX\r\n$2\r\n10\r\n";
        let command = Command::from_bytes(input).unwrap();
        assert_eq!(
            command,
            Command::Set("key".to_string(), "value".to_string(), Some(10))
        );
    }

    #[test]
    fn test_set_command_without_expiry() {
        let input = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
        let command = Command::from_bytes(input).unwrap();
        assert_eq!(
            command,
            Command::Set("key".to_string(), "value".to_string(), None)
        );
    }

    #[test]
    fn test_get_command() {
        let input = b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n";
        let command = Command::from_bytes(input).unwrap();
        assert_eq!(command, Command::Get("key".to_string()));
    }

    #[test]
    fn test_invalid_command() {
        let input = b"*1\r\n$4\r\nINVALID\r\n";
        let result = Command::from_bytes(input);
        assert!(result.is_err());
    }
}
