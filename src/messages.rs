use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MessageBody {
    // Client Requests
    Init {
        msg_id: u32,
        node_id: String,
        node_ids: Vec<String>,
    },
    Echo {
        msg_id: u32,
        echo: String,
    },
    Generate {
        msg_id: u32,
    },
    Broadcast {
        msg_id: u32,
        message: u32,
    },
    Read {
        msg_id: u32,
    },
    Topology {
        msg_id: u32,
        topology: HashMap<String, Vec<String>>,
    },

    // Client Responses
    InitOk {
        in_reply_to: u32,
    },
    EchoOk {
        msg_id: u32,
        in_reply_to: u32,
        echo: String,
    },
    GenerateOk {
        id: i64,
        in_reply_to: u32,
    },
    BroadcastOk {
        in_reply_to: u32,
    },
    ReadOk {
        in_reply_to: u32,
        messages: HashSet<u32>,
    },
    TopologyOk {
        in_reply_to: u32,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub src: String,
    pub dest: String,
    pub body: MessageBody,
}

pub fn send_message(message: &Message) {
    let stdout = std::io::stdout().lock();
    serde_json::to_writer(stdout, message)
        .expect("writing a serialized messaged to stdout shouldn't fail")
}
