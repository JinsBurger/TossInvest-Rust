use crate::toss_stomper::TossStomper;

use std::str::Bytes;
use std::time::Duration;
use std::{
    str::FromStr
};


use std::sync::Arc;
use reqwest::header::HeaderValue;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{Method, Request};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use tokio::sync::mpsc;

const TOSS_WS_URL: &str = "wss://realtime-socket.tossinvest.com/ws";

pub struct TossWebSock {
    conn_id: String,
    dev_id: String,
    utk_id: String,
    hook: fn(Vec<u8>),
    stomper: TossStomper,
    send_mpsc_tx: Option<mpsc::Sender<Vec<u8>>>
}

impl TossWebSock {
    pub fn new(conn_id: String, dev_id: String, utk_id: String, hook: fn(Vec<u8>)) -> Self {
        let stomper = TossStomper::new(conn_id.clone(), dev_id.clone(), utk_id.clone());
        Self { 
            conn_id: conn_id, 
            dev_id: dev_id, 
            utk_id: utk_id, 
            hook: hook,
            stomper: stomper,
            send_mpsc_tx: None
        }
    }

    pub async fn start(&mut self) {
        let url = url::Url::parse(TOSS_WS_URL).unwrap();
        let mut req = url.into_client_request().unwrap();
        let headers = req.headers_mut();

        headers.insert("Sec-WebSocket-Protocol", HeaderValue::from_str("v12.stomp, v11.stomp, v10.stomp").unwrap());
        let (ws, _) = connect_async(req).await.expect("Failed to connect Toss websocket");

        let (mut send, mut recv) = ws.split();
        let (mpsc_tx, mut mpsc_rx)= mpsc::channel(300);
        self.send_mpsc_tx = Some(mpsc_tx);

        let th1 = tokio::spawn(async move {
            Self::ws_recv_handler(recv).await;
        });
        let th2 = tokio::spawn(async move  {
            Self::ws_send_handler(send, mpsc_rx).await;
        });

        tokio::time::sleep(Duration::from_secs(3)).await;
        let connect_data = self.stomper.connect();
        self.send_mpsc(connect_data).await;
    }

    
    async fn ws_recv_handler(mut rx_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>) {
        while let Some(msg) = rx_stream.next().await {
            let msg = msg.unwrap();
            eprintln!("ws_recv_handler! {}", msg);
        }
    }

    async fn ws_send_handler(mut tx_stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, mut mpsc_rx: mpsc::Receiver<Vec<u8>>) {
        loop {
            let usr_msg = mpsc_rx.recv().await.unwrap();
            eprintln!("ws_send_handler! {}", usr_msg.len());
            tx_stream.send(Message::binary(usr_msg)).await.expect("ws_send_handler: Failed to send");
            
        }
    }

    async fn send_mpsc(&mut self, data: Vec<u8>) {
        self.send_mpsc_tx.as_mut().unwrap().send(data).await.expect("Failed to send a mpsc message");
    }


}