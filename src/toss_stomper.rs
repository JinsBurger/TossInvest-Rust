pub struct TossStomper {
    conn_id: String,
    dev_id: String,
    utk_id: String,
}


impl TossStomper {
    pub fn new(conn_id: String, dev_id: String, utk_id: String) -> Self {
        Self {
            conn_id: conn_id,
            dev_id: dev_id,
            utk_id: utk_id
        }
    }

    pub fn connect(&mut self) -> Vec<u8> {
        format!(
            "CONNECT\ndevice-id:{}\nconnection-id:{}\nauthorization:{}\naccept-version:1.2,1.1,1.0\nheart-beat:5000,5000\n\n\x00\n",
            self.dev_id, self.conn_id, self.utk_id
        ).into_bytes()
    }
}