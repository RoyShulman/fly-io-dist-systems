use thiserror::Error;

use crate::{
    messages::{InputMessage, InputMessageBody, OutputMessage, OutputMessageBody},
    unique_id::SnowflakeIdGenerator,
};

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
    fn handle_message(&mut self, message: InputMessage) -> Result<OutputMessage, HandlerError>;
}

///
/// Handler for `init` messages
#[derive(Debug, Default)]
pub struct UninitHandler {
    machine_id: Option<u16>,
}

impl UninitHandler {
    pub fn new() -> Self {
        Self { machine_id: None }
    }

    pub fn into_initialized_handler(&self) -> Option<InitializedHandler> {
        match self.machine_id {
            Some(id) => Some(InitializedHandler::new(id)),
            None => None,
        }
    }
}

impl Handler for UninitHandler {
    fn handle_message(&mut self, message: InputMessage) -> Result<OutputMessage, HandlerError> {
        let (node_id, msg_id) = match message.body {
            InputMessageBody::Init {
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

        Ok(OutputMessage {
            src: message.dest,
            dest: message.src,
            body: OutputMessageBody::InitOk {
                in_reply_to: msg_id,
            },
        })
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
}

impl InitializedHandler {
    pub fn new(machine_id: u16) -> Self {
        let unique_id_generator = SnowflakeIdGenerator::new(machine_id, 0);
        Self {
            unique_id_generator,
        }
    }
}

impl Handler for InitializedHandler {
    fn handle_message(&mut self, message: InputMessage) -> Result<OutputMessage, HandlerError> {
        let body = match message.body {
            InputMessageBody::Init { .. } => {
                return Err(HandlerError::UnprocessableMessage(
                    "initialized handler shouldn't accept init mesage".to_string(),
                ))
            }
            InputMessageBody::Echo { msg_id, echo } => OutputMessageBody::EchoOk {
                msg_id,
                in_reply_to: msg_id,
                echo,
            },
            InputMessageBody::Generate { msg_id } => {
                let id = self.unique_id_generator.generate().get();
                OutputMessageBody::GenerateOk {
                    id,
                    in_reply_to: msg_id,
                }
            }
        };

        Ok(OutputMessage {
            src: message.dest,
            dest: message.src,
            body,
        })
    }
}
