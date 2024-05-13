use crate::{
    messages::{send_message, Message, MessageBody},
    unique_id::SnowflakeIdGenerator,
};
use rand::seq::IteratorRandom;
use std::collections::{hash_map::Entry, HashMap, HashSet};
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

enum PendingSentMessages {
    /// An inform broadcast was sent to a neihbor. When he replies with ok we know he got the message.
    InformBroadcast { messages: HashSet<u32>, dst: String },
}

///
/// Handler that is initialized. This means this node accepts the `init` message,
/// responded and is ready to handle other data messages
pub struct InitializedHandler {
    unique_id_generator: SnowflakeIdGenerator,
    node_id: String,
    neighbors: Vec<String>,

    current_msg_id: u32,

    /// Holds all the messages seen so far
    messages: HashSet<u32>,

    known_messages_to_neighbors: HashMap<String, HashSet<u32>>,
    pending_messages_sent: HashMap<u32, PendingSentMessages>,
}

impl InitializedHandler {
    const NUM_RANDOM_NEIGHBORS_TO_INFORM: usize = 10;

    pub fn new(machine_id: u16) -> Self {
        let unique_id_generator = SnowflakeIdGenerator::new(machine_id, 0);
        Self {
            unique_id_generator,
            neighbors: Vec::new(),
            node_id: format!("n{machine_id}"),
            messages: HashSet::new(),
            known_messages_to_neighbors: HashMap::new(),
            pending_messages_sent: HashMap::new(),
            current_msg_id: 0,
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
        for neighbor in &self.neighbors {
            self.known_messages_to_neighbors
                .insert(neighbor.clone(), HashSet::new());
        }
    }

    fn send_inform_broadcast_to_neighbors(&mut self) {
        let neighbors_to_inform = self.neighbors.iter().choose_multiple(
            &mut rand::thread_rng(),
            Self::NUM_RANDOM_NEIGHBORS_TO_INFORM,
        );

        for neighbor in neighbors_to_inform.into_iter().cloned() {
            let known_by_other = self
                .known_messages_to_neighbors
                .entry(neighbor.clone())
                .or_default();

            let messages: HashSet<_> = self.messages.difference(known_by_other).copied().collect();
            if messages.is_empty() {
                // no need to send empty messages
                continue;
            }

            let body = MessageBody::InformNewBroadcast {
                msg_id: self.current_msg_id,
                messages: messages.clone(),
            };
            let message = Message {
                src: self.node_id.clone(),
                dest: neighbor.clone(),
                body,
            };
            send_message(&message);
            match self.pending_messages_sent.entry(self.current_msg_id) {
                Entry::Occupied(_) => eprintln!(
                    "pending message with the same message id ({}) was already sent!",
                    self.current_msg_id
                ),
                Entry::Vacant(e) => {
                    let _ = e.insert(PendingSentMessages::InformBroadcast {
                        messages,
                        dst: neighbor,
                    });
                }
            };

            self.current_msg_id += 1;
        }
    }

    ///
    /// Choose a few neighbors in random and send them all the messages we know they haven't seen.
    pub fn handle_gossip_timer(&mut self) {
        self.send_inform_broadcast_to_neighbors()
    }

    ///
    /// A node sent us of the new messages he has seen so far.
    /// We should update the total messages set, and also the seen messages from this neighbor
    fn handle_inform_new_broadcast(
        &mut self,
        message_src: String,
        msg_id: u32,
        messages: HashSet<u32>,
    ) {
        self.messages.extend(messages.clone());

        match self.known_messages_to_neighbors.entry(message_src.clone()) {
            Entry::Occupied(mut entry) => entry.get_mut().extend(messages),
            Entry::Vacant(entry) => {
                eprintln!(
                    "Got a messages from an unknown neighbor {message_src}. This shouldn't happen"
                );
                entry.insert(messages);
            }
        };

        let body = MessageBody::InformNewBroadcastOk {
            in_reply_to: msg_id,
        };
        let response = Message {
            src: self.node_id.clone(),
            dest: message_src,
            body,
        };
        send_message(&response);
    }

    fn handle_response(&mut self, msg_id: u32) {
        let Some(pending_message) = self.pending_messages_sent.remove(&msg_id) else {
            eprintln!("Got an response message to a message that wasn't sent (msg_id = {msg_id})");
            return;
        };

        self.handle_pending_message(pending_message);
    }

    fn handle_pending_message(&mut self, message: PendingSentMessages) {
        match message {
            PendingSentMessages::InformBroadcast { messages, dst } => self
                .known_messages_to_neighbors
                .entry(dst)
                .and_modify(|known_messages| known_messages.extend(messages))
                .or_default(),
        };
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
            MessageBody::InformNewBroadcast { msg_id, messages } => {
                self.handle_inform_new_broadcast(message.src, msg_id, messages)
            }
            MessageBody::InformNewBroadcastOk { in_reply_to } => self.handle_response(in_reply_to),
        };

        Ok(())
    }
}
