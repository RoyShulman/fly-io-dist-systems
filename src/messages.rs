use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum InputMessageBody {
    Init {
        msg_id: u32,
        node_id: String,
        node_ids: Vec<String>,
    },
    Echo {
        msg_id: u32,
        echo: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct InputMessage {
    pub src: String,
    pub dest: String,
    pub body: InputMessageBody,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum OutputMessageBody {
    InitOk {
        in_reply_to: u32,
    },
    EchoOk {
        msg_id: u32,
        in_reply_to: u32,
        echo: String,
    },
}

#[derive(Debug, Serialize)]
pub struct OutputMessage {
    pub src: String,
    pub dest: String,
    pub body: OutputMessageBody,
}
