use crate::{
    messages::{send_message, Message, MessageBody},
    unique_id::SnowflakeIdGenerator,
};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Unable to process message of type: {0}")]
    UnprocessableMessage(String),
    #[error("Got an invalid machine ID: {0}")]
    InvalidMachineId(String),
    #[error("Got message in an invalid state: {0}")]
    ReceivedMessageInInvalidState(String),
}

pub trait Handler {
    fn handle_message(&mut self, message: Message) -> Result<(), HandlerError>;
}

///
/// Handler for `init` messages
#[derive(Debug)]
pub struct UninitHandler {
    machine_id: Option<u16>,
}

impl UninitHandler {
    pub fn new() -> Self {
        Self { machine_id: None }
    }

    pub fn get_initialized_handler(&self) -> Option<InitializedHandler> {
        self.machine_id.map(InitializedHandler::new)
    }
}

impl Handler for UninitHandler {
    fn handle_message(&mut self, message: Message) -> Result<(), HandlerError> {
        let (node_id, msg_id) = match message.body {
            MessageBody::Init {
                msg_id,
                node_id,
                node_ids: _,
            } => (node_id, msg_id),
            _ => return Err(HandlerError::UnprocessableMessage("we can only handle init. should probably add the message type to this error string".to_string())),
        };

        let machine_id = parse_node_id(node_id)?;
        if self.machine_id.is_some() {
            return Err(HandlerError::ReceivedMessageInInvalidState(
                "uninitialized handler already received an init message".to_string(),
            ));
        }

        self.machine_id.replace(machine_id);
        let response = Message {
            src: message.dest,
            dest: message.src,
            body: MessageBody::InitOk {
                in_reply_to: msg_id,
            },
        };

        send_message(&response);

        Ok(())
    }
}

///
/// Node id should be "n<number>"
fn parse_node_id(node_id: String) -> Result<u16, HandlerError> {
    if node_id.len() < 2 {
        return Err(HandlerError::InvalidMachineId(node_id));
    }

    node_id[1..]
        .parse()
        .map_err(|_| HandlerError::InvalidMachineId(node_id))
}

///
/// Handler that is initialized. This means this node accepts the `init` message,
/// responded and is ready to handle other data messages
pub struct InitializedHandler {
    unique_id_generator: SnowflakeIdGenerator,
    node_id: String,
    neighbors: Vec<String>,

    /// Holds all the messages seen so far
    messages: HashSet<u32>,
}

impl InitializedHandler {
    pub fn new(machine_id: u16) -> Self {
        let unique_id_generator = SnowflakeIdGenerator::new(machine_id, 0);
        Self {
            unique_id_generator,
            neighbors: Vec::new(),
            node_id: format!("n{machine_id}"),
            messages: HashSet::new(),
        }
    }

    fn handle_topology(&mut self, mut topology: HashMap<String, Vec<String>>) {
        let Some(neighbors) = topology.remove(&self.node_id) else {
            return;
        };
        eprintln!(
            "{} - updating topology with neighbors: {:?}",
            self.node_id, neighbors
        );
        self.neighbors = neighbors;
    }
}

impl Handler for InitializedHandler {
    fn handle_message(&mut self, message: Message) -> Result<(), HandlerError> {
        match message.body {
            MessageBody::Init { .. } => {
                return Err(HandlerError::UnprocessableMessage(
                    "initialized handler shouldn't accept init mesage".to_string(),
                ))
            }
            MessageBody::Echo { msg_id, echo } => {
                let body = MessageBody::EchoOk {
                    msg_id,
                    in_reply_to: msg_id,
                    echo,
                };
                let message = Message {
                    src: message.dest,
                    dest: message.src,
                    body,
                };
                send_message(&message);
            }
            MessageBody::Generate { msg_id } => {
                let id = self.unique_id_generator.generate().get();
                let body = MessageBody::GenerateOk {
                    id,
                    in_reply_to: msg_id,
                };
                let message = Message {
                    src: message.dest,
                    dest: message.src,
                    body,
                };
                send_message(&message);
            }
            MessageBody::Broadcast {
                msg_id,
                message: value,
            } => {
                self.messages.insert(value);

                let body = MessageBody::BroadcastOk {
                    in_reply_to: msg_id,
                };
                let message = Message {
                    src: message.dest,
                    dest: message.src,
                    body,
                };
                send_message(&message);
            }
            MessageBody::Read { msg_id } => {
                let body = MessageBody::ReadOk {
                    in_reply_to: msg_id,
                    messages: self.messages.clone(),
                };
                let message = Message {
                    src: message.dest,
                    dest: message.src,
                    body,
                };
                send_message(&message);
            }
            MessageBody::Topology { msg_id, topology } => {
                self.handle_topology(topology);
                let body = MessageBody::TopologyOk {
                    in_reply_to: msg_id,
                };
                let message = Message {
                    src: message.dest,
                    dest: message.src,
                    body,
                };
                send_message(&message);
            }
            MessageBody::InitOk { .. }
            | MessageBody::EchoOk { .. }
            | MessageBody::GenerateOk { .. }
            | MessageBody::BroadcastOk { .. }
            | MessageBody::ReadOk { .. }
            | MessageBody::TopologyOk { .. } => (),
        };

        Ok(())
    }
}
