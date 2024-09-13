#![allow(unused_imports)]
use std::io::{Read, Write};

use anyhow::Context;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

mod resp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    tracing::info!("Server listening on 127.0.0.1:6379");

    loop {
        let (mut socket, _addr) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = [0; 1024];
            loop {
                let n = socket
                    .read(&mut buf)
                    .await
                    .context("Failed to read from socket")
                    .unwrap();

                if n == 0 {
                    return;
                }

                tracing::info!("Received: {:?}", &buf[..n]);

                socket
                    .write_all(b"+PONG\r\n")
                    .await
                    .context("Failed to write to socket")
                    .unwrap();
            }
        });
    }
}
