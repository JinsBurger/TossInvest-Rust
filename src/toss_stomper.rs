use std::{collections::HashMap, hash::Hash};

pub struct TossStomper {
    conn_id: String,
    dev_id: String,
    utk_id: String,
    stock_id_map: HashMap<String, u32>,
    cur_stk_id: u32
}


impl TossStomper {
    pub fn new(conn_id: String, dev_id: String, utk_id: String) -> Self {
        Self {
            conn_id: conn_id,
            dev_id: dev_id,
            utk_id: utk_id,
            stock_id_map: HashMap::new(),
            cur_stk_id: 0
        }
    }

    pub fn connect(&mut self) -> Vec<u8> {
        format!(
            "CONNECT\ndevice-id:{}\nconnection-id:{}\nauthorization:{}\naccept-version:1.2,1.1,1.0\nheart-beat:5000,5000\n\n\x00\n",
            self.dev_id, self.conn_id, self.utk_id
        ).into_bytes()
    }

    pub fn subscribe(&mut self, stock_code: String) -> Vec<u8> {
        let dest = format!("/topic/quote/stock/{}", stock_code);

    }
}