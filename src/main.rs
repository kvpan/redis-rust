#![allow(unused_imports)]
use std::io::{Read, Write};

use anyhow::Context;
use commands::Command;
use resp::RespValue;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

mod commands;
mod cursor;
mod resp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    tracing::info!("Server listening on 127.0.0.1:6379");

    loop {
        let (mut socket, _addr) = listener.accept().await?;

        async fn send(socket: &mut tokio::net::TcpStream, msg: RespValue) -> anyhow::Result<()> {
            socket
                .write_all(&msg.as_bytes())
                .await
                .context("Failed to write to socket")?;
            Ok(())
        }

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

                match Command::from_bytes(&buf[..n]) {
                    Ok(Command::Ping) => {
                        tracing::info!("Received PING");
                        let reply = RespValue::SimpleString("PONG".to_string());
                        send(&mut socket, reply).await.unwrap();
                    }
                    Ok(Command::Echo(arg)) => {
                        tracing::info!("Received ECHO: {:?}", arg);
                        let reply = RespValue::SimpleString(arg);
                        send(&mut socket, reply).await.unwrap();
                    }
                    Err(e) => {
                        tracing::warn!("Error: {:?}", e);
                        let reply = RespValue::Error("unknown command".to_string());
                        send(&mut socket, reply).await.unwrap();
                    }
                }
            }
        });
    }
}
