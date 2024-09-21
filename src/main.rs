#![allow(unused_imports)]
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::Arc,
};

use anyhow::Context;
use commands::Command;
use resp::RespValue;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex,
};

mod commands;
mod cursor;
mod kv;
mod resp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    kv::init();

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
                    .expect("Failed to read from socket");

                if n == 0 {
                    return;
                }

                match Command::from_bytes(&buf[..n]) {
                    Ok(Command::Ping) => {
                        tracing::info!("Received PING");
                        let reply = RespValue::SimpleString("PONG".to_string());
                        send(&mut socket, reply).await.expect("Failed to send PONG");
                    }
                    Ok(Command::Echo(arg)) => {
                        tracing::info!("Received ECHO: {:?}", arg);
                        let reply = RespValue::BulkString(arg);
                        send(&mut socket, reply).await.expect("Failed to send ECHO");
                    }
                    Ok(Command::Set(key, value, expiry)) => {
                        tracing::info!(?key, ?value, ?expiry, "Received SET");
                        kv::set(&key, value, expiry).await;
                        let reply = RespValue::SimpleString("OK".to_string());
                        send(&mut socket, reply).await.expect("Failed to send OK");
                    }
                    Ok(Command::Get(key)) => {
                        tracing::info!(?key, "Received GET");
                        let value = kv::get(&key).await;
                        match value {
                            Some(value) => {
                                let reply = RespValue::BulkString(value.to_string());
                                send(&mut socket, reply).await.expect("Failed to send GET");
                            }
                            None => {
                                let reply = RespValue::NullBulkString;
                                send(&mut socket, reply).await.expect("Failed to send GET");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Error: {:?}", e);
                        let reply = RespValue::Error("unknown command".to_string());
                        send(&mut socket, reply)
                            .await
                            .expect("Failed to send ERROR");
                    }
                }
            }
        });
    }
}
