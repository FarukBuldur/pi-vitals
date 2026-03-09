use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};
use tracing::{error, info};

use crate::vitals::VitalsCollector;

pub async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    info!("WebSocket client connected");

    let collector = Arc::new(Mutex::new(VitalsCollector::new()));
    let mut tick = interval(Duration::from_secs(2));

    loop {
        tick.tick().await;

        let vitals = {
            let mut c = collector.lock().unwrap();
            c.collect()
        };

        match serde_json::to_string(&vitals) {
            Ok(json) => {
                if socket.send(Message::Text(json.into())).await.is_err() {
                    info!("WebSocket client disconnected");
                    break;
                }
            }
            Err(e) => {
                error!("Failed to serialize vitals: {}", e);
            }
        }
    }
}