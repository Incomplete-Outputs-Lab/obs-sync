use anyhow::{Context, Result};
use obws::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OBSConnectionConfig {
    pub host: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OBSConnectionStatus {
    pub connected: bool,
    pub obs_version: Option<String>,
    pub obs_websocket_version: Option<String>,
}

#[derive(Clone)]
pub struct OBSClient {
    client: Arc<RwLock<Option<Client>>>,
    config: Arc<RwLock<Option<OBSConnectionConfig>>>,
}

impl OBSClient {
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn connect(&self, config: OBSConnectionConfig) -> Result<()> {
        let host = config.host.clone();
        let port = config.port;
        let password = config.password.clone();

        let client = Client::connect(host, port, password)
            .await
            .context("Failed to connect to OBS WebSocket")?;

        *self.client.write().await = Some(client);
        *self.config.write().await = Some(config);

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        let mut client_lock = self.client.write().await;
        if let Some(client) = client_lock.take() {
            drop(client);
        }
        *self.config.write().await = None;
        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        self.client.read().await.is_some()
    }

    pub async fn get_status(&self) -> OBSConnectionStatus {
        let client_lock = self.client.read().await;

        if let Some(client) = client_lock.as_ref() {
            // Try to get version info
            if let Ok(version) = client.general().version().await {
                return OBSConnectionStatus {
                    connected: true,
                    obs_version: Some(version.obs_version.to_string()),
                    obs_websocket_version: Some(version.obs_web_socket_version.to_string()),
                };
            }
        }

        OBSConnectionStatus {
            connected: false,
            obs_version: None,
            obs_websocket_version: None,
        }
    }

    pub fn get_client_arc(&self) -> Arc<RwLock<Option<Client>>> {
        self.client.clone()
    }
}

impl Default for OBSClient {
    fn default() -> Self {
        Self::new()
    }
}
