use rand::{distributions::Alphanumeric, Rng};
use md5;
use uuid::Uuid;
use reqwest::header::{HeaderMap, HeaderValue};

pub mod toss_websock;
use toss_websock::TossWebSock;



fn get_connection_headers() -> (String, String, String) {
    const url: &str = "https://wts-api.tossinvest.com/api/v3/init";
    let rand_str: String = rand::thread_rng().sample_iter(Alphanumeric).take(35).map(char::from).collect();
    let device_id: String = format!("WTS-{:x}", md5::compute(rand_str.as_bytes()));
    let connection_id = Uuid::new_v4();

    let client = reqwest::blocking::Client::new();
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
        .send().unwrap();

    
    let set_cookies = resp.headers().get_all("Set-Cookie");
    let mut utk_id = "";
    for v in set_cookies.iter() {
        if v.to_str().unwrap().starts_with("UTK=") {
            utk_id = v.to_str().unwrap().split("=").nth(1).unwrap();
            break;
        }
    }

    (connection_id.to_string(), device_id, utk_id.to_string())
}

fn connect(hook: fn(Vec<u8>)) -> TossWebSock {
    let (conn_id, dev_id, utk_id) =  get_connection_headers();
    let toss_sock = TossWebSock::new(conn_id, dev_id, utk_id, hook);
    
    toss_sock
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conn_test() {
        fn hook(data: Vec<u8>) {
            println!("Len: {}", data.len());
        }
        connect(hook);

        loop {
            let mut str = String::new();
            std::io::stdin().read_line(&mut str).unwrap();
        }
    }
}
