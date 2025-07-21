use crate::toss_stomper::TossStomper;
use crate::{ts_debug};

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
use tokio::sync::{mpsc, oneshot};


type WsStreamRecv = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
type WsStreamSend = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

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
        let (toss_mpsc_tx, mut toss_mpsc_rx)= mpsc::channel(300);
        self.send_mpsc_tx = Some(toss_mpsc_tx.clone());


        let (init_noti_tx, init_noti_rx) = oneshot::channel();

        let th1 = tokio::spawn(async move {
            Self::ws_recv_handler(recv, Some(toss_mpsc_tx), Some(init_noti_tx)).await;
        });
        let th2 = tokio::spawn(async move  {
            Self::ws_send_handler(send, toss_mpsc_rx).await;
        });

        let connect_data = self.stomper.connect();
        self.send_mpsc(connect_data).await;
        ts_debug!("Wait for initialzation from Toss Server...");
        init_noti_rx.await.expect("Failed to initialize websocket");
        ts_debug!("Success to initialize connection");
    }


    /* -- Public callable stock functions -- */
    pub async fn register_stock(&mut self, stock_code: String) {
        ts_debug!("Register Stock");
    }


    /* -- Private functions --  */
    async fn ws_recv_handler(mut toss_ws_rx: WsStreamRecv, toss_mpsc_tx: Option<mpsc::Sender<Vec<u8>>>, mut init_noti_tx: Option<oneshot::Sender<u8>>) {
        while let Some(msg) = toss_ws_rx.next().await {
            ts_debug!("ws_recv_handler! {} {} ", msg.as_ref().unwrap().clone(), msg.as_ref().unwrap().clone().len());
            let msg = msg.unwrap().into_text().unwrap();
            
            if msg.len() == 1  && let Some(tx) = toss_mpsc_tx.as_ref() {
                // PING(HeartBeat) 
                ts_debug!("Send HeartBeat");
                tx.send("\n".as_bytes().to_vec()).await.expect("Failed to send HeartBeart");
            } else {
                // MESSAge, RECEIPT, DESCRIBE
                let parts: Vec<&str> = msg.splitn(2, "\n\n").collect();
                let header = parts[0];
                let body = parts[1];

                if header.starts_with("CONNECTED") {
                    if let Some(noti) = init_noti_tx.take() {
                        noti.send(0).expect("Fail to notify a intialization");
                    }
                }
            }
        }
    }

    async fn ws_send_handler(mut toss_ws_tx: WsStreamSend, mut toss_mpsc_rx: mpsc::Receiver<Vec<u8>>) {
        loop {
            let usr_msg = toss_mpsc_rx.recv().await.unwrap();
            ts_debug!("ws_send_handler! {}", usr_msg.len());
            toss_ws_tx.send(Message::binary(usr_msg)).await.expect("ws_send_handler: Failed to send");
            
        }
    }

    async fn send_mpsc(&mut self, data: Vec<u8>) {
        self.send_mpsc_tx.as_mut().unwrap().send(data).await.expect("Failed to send a mpsc message");
    }
}