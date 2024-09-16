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
mod resp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    tracing::info!("Server listening on 127.0.0.1:6379");

    let state = Arc::new(Mutex::new(HashMap::<String, String>::new()));

    loop {
        let (mut socket, _addr) = listener.accept().await?;

        async fn send(socket: &mut tokio::net::TcpStream, msg: RespValue) -> anyhow::Result<()> {
            socket
                .write_all(&msg.as_bytes())
                .await
                .context("Failed to write to socket")?;
            Ok(())
        }

        let state = Arc::clone(&state);
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
                        let reply = RespValue::BulkString(arg);
                        send(&mut socket, reply).await.unwrap();
                    }
                    Ok(Command::Set(key, value)) => {
                        tracing::info!("Received SET: {:?} {:?}", key, value);
                        let reply = RespValue::SimpleString("OK".to_string());
                        let mut state = state.lock().await;
                        state.insert(key, value);
                        send(&mut socket, reply).await.unwrap();
                    }
                    Ok(Command::Get(key)) => {
                        tracing::info!("Received GET: {:?}", key);
                        let state = state.lock().await;
                        match state.get(&key) {
                            Some(value) => {
                                let reply = RespValue::BulkString(value.to_string());
                                send(&mut socket, reply).await.unwrap();
                            }
                            None => {
                                let reply = RespValue::Null;
                                send(&mut socket, reply).await.unwrap();
                            }
                        }
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
