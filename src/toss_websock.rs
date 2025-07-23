use crate::{ts_debug};

use crate::toss_stomper::{TossStomper, TossStompResponseType};
use std::str::Bytes;
use std::time::Duration;
use std::{
    str::FromStr
};

use serde::Deserialize;
use serde_json;

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


#[derive(Debug, Deserialize)]
pub struct Trade {
    pub code: String,
    pub dt: String,
    pub session: String,
    pub currency: String,
    pub base: f64,
    pub close: f64,
    pub baseKrw: f64,
    pub closeKrw: f64,
    pub volume: f64,
    pub tradeType: String,
    pub changeType: String,
    pub tradingStrength: f64,
    pub cumulativeVolume: f64,
    pub cumulativeAmount: f64,
    pub cumulativeAmountKrw: f64,
}

pub trait TradeHandler: Send + Sync {
    fn handle_trade(&mut self, trade: Trade);
}

pub struct TossWebSock {
    conn_id: String,
    dev_id: String,
    utk_id: String,
    responser: Option<Box<dyn TradeHandler>>,
    stomper: TossStomper,
    send_mpsc_tx: Option<mpsc::Sender<Vec<u8>>>
}


impl TossWebSock {
    pub fn new(conn_id: String, dev_id: String, utk_id: String, responser: Box<dyn TradeHandler>) -> Self {
        let stomper = TossStomper::new(conn_id.clone(), dev_id.clone(), utk_id.clone());
        Self { 
            conn_id: conn_id, 
            dev_id: dev_id, 
            utk_id: utk_id, 
            responser: Some(responser),
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
        let responser = self.responser.take().unwrap();
        let th1 = tokio::spawn(async move {
            Self::ws_recv_handler(recv, Some(toss_mpsc_tx), Some(init_noti_tx), responser).await;
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
        ts_debug!("Register Stock: {}", stock_code);
        let subscribe_data = self.stomper.subscribe(stock_code);
        self.send_mpsc(subscribe_data).await;
    }

    pub async fn unregister_stock(&mut self, stock_code: String) {
        ts_debug!("Unregister Stock: {}", stock_code);
        let unsubscribe_data = self.stomper.unsubscribe(stock_code);
        self.send_mpsc(unsubscribe_data).await;
    }


    /* -- Private functions --  */

    /// Processes a single stock trade event.
    async fn ws_recv_handler(mut toss_ws_rx: WsStreamRecv, toss_mpsc_tx: Option<mpsc::Sender<Vec<u8>>>, mut init_noti_tx: Option<oneshot::Sender<u8>>, mut responser: Box<dyn TradeHandler>) {
        while let Some(msg) = toss_ws_rx.next().await {
            let msg = msg.unwrap().into_text().unwrap();
            
            if msg.len() == 1  && let Some(tx) = toss_mpsc_tx.as_ref() {
                // PING(HeartBeat) 
                ts_debug!("Send HeartBeat");
                tx.send("\n".as_bytes().to_vec()).await.expect("Failed to send HeartBeart");
            } else {
                let parts: Vec<&str> = msg.splitn(2, "\n\n").collect();
                let header = parts[0];

                match TossStompResponseType::from_header(header) {
                    TossStompResponseType::Connected => {
                        if let Some(noti) = init_noti_tx.take() {
                            noti.send(0).expect("Fail to notify a intialization");
                        }
                    }
                    TossStompResponseType::Message => {
                        let body = parts[1].replace("\x00", "");
                        ts_debug!("MESSAGE: {}",  body);
                        let trade_info = serde_json::from_str::<Trade>(body.as_str()).expect("Failed to parse json");
                        responser.handle_trade(trade_info);
                    }
                    TossStompResponseType::Receipt => {
                        ts_debug!("RECEIPT: {}",  msg);
                    }
                    TossStompResponseType::Unknown => {
                        ts_debug!("Unexpected header: {}",  msg);
                    }
                }
            }
        }
    }

    async fn ws_send_handler(mut toss_ws_tx: WsStreamSend, mut toss_mpsc_rx: mpsc::Receiver<Vec<u8>>) {
        loop {
            let usr_msg = toss_mpsc_rx.recv().await.unwrap();
            toss_ws_tx.send(Message::binary(usr_msg)).await.expect("ws_send_handler: Failed to send");
            
        }
    }

    async fn send_mpsc(&mut self, data: Vec<u8>) {
        self.send_mpsc_tx.as_mut().unwrap().send(data).await.expect("Failed to send a mpsc message");
    }
}