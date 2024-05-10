use std::io::{self, BufRead};

use messages::{InputMessage, InputMessageBody, OutputMessage, OutputMessageBody};

mod messages;

fn main() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        handle_single_line(&line.unwrap());
    }
}

fn handle_single_line(line: &str) {
    let message: InputMessage = serde_json::from_str(line).unwrap();
    let body = match message.body {
        InputMessageBody::Echo { msg_id, echo } => OutputMessageBody::EchoOk {
            msg_id,
            in_reply_to: msg_id,
            echo,
        },
        InputMessageBody::Init {
            msg_id,
            node_id: _,
            node_ids: _,
        } => OutputMessageBody::InitOk {
            in_reply_to: msg_id,
        },
    };

    let output_message = OutputMessage {
        src: message.dest,
        dest: message.src,
        body,
    };

    let serialized =
        serde_json::to_string(&output_message).expect("output message is serializable");
    println!("{}", serialized);
}
