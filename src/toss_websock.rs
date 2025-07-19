use std::{
    thread::{spawn}
};


use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::http::{Method, Request};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};

const TOSS_WS_URL: &str = "wss://realtime-socket.tossinvest.com/ws";

pub struct TossWebSock {
    conn_id: String,
    device_id: String,
    utk_id: String,
    hook: fn(Vec<u8>),
    ws: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>
}

impl TossWebSock {
    pub fn new(conn_id: String, device_id: String, utk_id: String, hook: fn(Vec<u8>)) -> Self {
        Self { 
            conn_id: conn_id, 
            device_id: device_id, 
            utk_id: utk_id, 
            hook: hook,
            ws: None
        }
    }

    pub async fn start(self: Arc<Self>) {
        let req = Request::builder()
            .uri(TOSS_WS_URL)
            .header("Sec-WebSocket-Protocol", "v12.stomp, v11.stomp, v10.stomp")
            .body(()).unwrap();

        let (ws, _) = connect_async(req).await.expect("Failed to connect Toss websocket");
        let (mut send, mut recv) = ws.split();
        spawn(move || {
            self.ws_recv_handler(recv);
        });
    }

    async fn ws_recv_handler(self: Arc<Self>, mut recv: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>) {
        while let Some(msg) = recv.next().await {
                match msg.unwrap() {
                    Message::Text(text) => {
                        println!("Recv: {}", text);
                    }
                    _ => {  println!("Recv2");}
                };
            }
    }
}