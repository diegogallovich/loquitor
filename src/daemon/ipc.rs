use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tracing::{debug, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub command: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub status: String,
    pub data: serde_json::Value,
}

/// Run the IPC server on a Unix socket.
/// Handles JSON request/response per connection.
pub async fn run_ipc_server(socket_path: &Path) -> Result<()> {
    // Remove stale socket file if present
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;
    info!(path = %socket_path.display(), "IPC server listening");

    loop {
        let (stream, _addr) = listener.accept().await?;

        tokio::spawn(async move {
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            if let Err(e) = reader.read_line(&mut line).await {
                warn!(error = %e, "Failed to read IPC request");
                return;
            }

            let response = match serde_json::from_str::<IpcRequest>(&line) {
                Ok(req) => handle_request(req).await,
                Err(e) => IpcResponse {
                    status: "error".into(),
                    data: serde_json::json!({"message": format!("Invalid JSON: {e}")}),
                },
            };

            let json = match serde_json::to_string(&response) {
                Ok(j) => j,
                Err(e) => {
                    warn!(error = %e, "Failed to serialize IPC response");
                    return;
                }
            };

            if writer.write_all(json.as_bytes()).await.is_err() {
                return;
            }
            let _ = writer.write_all(b"\n").await;
        });
    }
}

async fn handle_request(req: IpcRequest) -> IpcResponse {
    debug!(command = %req.command, "IPC request received");
    match req.command.as_str() {
        "status" => IpcResponse {
            status: "ok".into(),
            data: serde_json::json!({"running": true}),
        },
        "lanes" => IpcResponse {
            status: "ok".into(),
            data: serde_json::json!({"lanes": []}),
        },
        "shutdown" => {
            info!("Shutdown requested via IPC");
            // Exit process; systemd-style restart is up to init
            std::process::exit(0);
        }
        other => IpcResponse {
            status: "error".into(),
            data: serde_json::json!({"message": format!("Unknown command: {other}")}),
        },
    }
}
