#![allow(unused_imports)]
use std::io::{Read, Write};

use anyhow::Context;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    println!("Server listening on 127.0.0.1:6379");

    loop {
        let (mut socket, _addr) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = [0; 1024];
            loop {
                let n = socket
                    .read(&mut buf)
                    .await
                    .expect("Failed to read from socket");

                if n == 0 {
                    return;
                }

                println!("Received: {:?}", &buf[..n]);

                socket
                    .write_all(b"+PONG\r\n")
                    .await
                    .expect("Failed to write to socket");
            }
        });
    }
}
