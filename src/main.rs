#![allow(unused_imports)]
use std::{
    io::{Read, Write},
    net::TcpListener,
};

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").context("Failed to bind to address")?;

    println!("Server listening on 127.0.0.1:6379");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => loop {
                let mut buf = [0; 1024];

                let n = stream
                    .read(&mut buf)
                    .context("Failed to read from stream")?;

                if n == 0 {
                    break;
                }

                println!("Received: {:?}", &buf[..n]);

                stream
                    .write_all(b"+PONG\r\n")
                    .context("Failed to write to stream")?;
            },
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}
