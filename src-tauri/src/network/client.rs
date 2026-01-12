use crate::sync::protocol::SyncMessage;
use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct SlaveClient {
    host: String,
    port: u16,
    ws_stream: Arc<RwLock<Option<WsStream>>>,
}

impl SlaveClient {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            ws_stream: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn connect(&self) -> Result<mpsc::UnboundedReceiver<SyncMessage>> {
        let url = format!("ws://{}:{}", self.host, self.port);
        let (ws_stream, _) = connect_async(&url)
            .await
            .context(format!("Failed to connect to {}", url))?;

        println!("Connected to master: {}", url);

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, rx) = mpsc::unbounded_channel();

        // Handle incoming messages
        tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<SyncMessage>(&text) {
                            Ok(sync_msg) => {
                                if tx.send(sync_msg).is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse sync message: {}", e);
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        // Send pong
                        let _ = ws_sender.send(Message::Pong(data)).await;
                    }
                    Ok(Message::Close(_)) => {
                        println!("Connection closed by master");
                        break;
                    }
                    Err(e) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(rx)
    }

    pub async fn is_connected(&self) -> bool {
        self.ws_stream.read().await.is_some()
    }

    pub async fn disconnect(&self) {
        let mut stream_lock = self.ws_stream.write().await;
        if let Some(mut stream) = stream_lock.take() {
            let _ = stream.close(None).await;
        }
    }
}
