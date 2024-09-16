use crate::cursor::Error as RespError;
use crate::resp::parse;
use crate::resp::RespValue;

pub enum Command {
    Ping,
    Echo(String),
    Set(String, String),
    Get(String),
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
            "PING" => Ok(Command::Ping),
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
            "SET" => {
                if arr.len() != 3 {
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

                Ok(Command::Set(key.to_string(), value.to_string()))
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
            _ => Err(RespError::InvalidInput(
                String::from_utf8(bytes.to_vec()).unwrap(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
