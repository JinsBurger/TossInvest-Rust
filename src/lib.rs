use rand::{distributions::Alphanumeric, Rng};
use md5;
use uuid::Uuid;
use reqwest::header::{HeaderMap, HeaderValue};
use tokio::io::{BufReader};

pub mod toss_stomper;
pub mod toss_websock;
use toss_websock::{TossWebSock, TradeHandler};
use std::panic::Location;

#[track_caller]
pub fn log_debug_inner(msg: String) {
    let loc = Location::caller();
    println!("[{}:{}] => {}",  loc.file(), loc.line(), msg);
}


#[macro_export]
macro_rules! ts_debug {
    ($msg:expr) => {
        #[cfg(feature = "debug")]
        {
            crate::log_debug_inner($msg.to_string());
        }
    };
    ($fmt:expr, $($args:tt)*) => {
        #[cfg(feature = "debug")]
        {
            crate::log_debug_inner(format!($fmt, $($args)*));
        }
    }
}

async fn get_connection_headers() -> (String, String, String) {
    const url: &str = "https://wts-api.tossinvest.com/api/v3/init";
    let rand_str: String = rand::thread_rng().sample_iter(Alphanumeric).take(35).map(char::from).collect();
    let device_id: String = format!("WTS-{:x}", md5::compute(rand_str.as_bytes()));
    let connection_id = Uuid::new_v4();

    let client = reqwest::Client::new();
    let resp = client.get(url)
        .header("accept", "*/*")
        .header("accept-encoding", "gzip, deflate, br, zstd")
        .header("accept-language", "ko-KR,ko;q=0.9")
        .header("app-version", "2024-12-26 18:33:54")
        .header("connection", "keep-alive")
        .header("cookie", HeaderValue::from_str(&format!("x-toss-distribution-id=53; deviceId={}", device_id)).unwrap())
        .header("host", "wts-api.tossinvest.com")
        .header("origin", "https://tossinvest.com")
        .header("referer", "https://tossinvest.com/")
        .header("sec-ch-ua", r#""Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24""#)
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", r#""macOS""#)
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-site")
        .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .send().await.unwrap();

    
    let set_cookies = resp.headers().get_all("Set-Cookie");
    let mut utk_id = "";
    for v in set_cookies.iter() {
        if v.to_str().unwrap().starts_with("UTK=") {
            utk_id = v.to_str().unwrap().split("=").nth(1).unwrap();
            break;
        }
    }

    assert_ne!(utk_id.len(), 0);
    utk_id = utk_id.split(";").next().unwrap();
    (connection_id.to_string(), device_id, utk_id.to_string())
}

pub async fn connect_toss(responsor: Box<dyn TradeHandler>) -> TossWebSock {
    let (conn_id, dev_id, utk_id) =  get_connection_headers().await;
    let toss_sock = TossWebSock::new(conn_id, dev_id, utk_id, responsor);
    toss_sock
}

#[cfg(test)]
mod tests {
    use crate::toss_websock::Trade;

    use super::*;

    #[tokio::test]
    async fn test() {
        struct TestResponser {}
        
        impl TestResponser {
            pub fn new() -> Self {
                Self {
                    
                }
            }
        }
        impl TradeHandler for TestResponser {
            fn handle_trade(&mut self, trade: Trade) {
                ts_debug!("{}: {}", trade.code, trade.changeType);
            }
        }

        let mut sock: TossWebSock = connect_toss(Box::new(TestResponser::new())).await;
        sock.start().await;
        sock.register_stock("AMX0231027004".to_string()).await;
        sock.register_stock("US20220809012".to_string()).await;


        tokio::time::sleep(std::time::Duration::from_secs(999_999)).await;
    }
}
