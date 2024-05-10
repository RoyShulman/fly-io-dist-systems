use std::io::{self, BufRead};

use handler::{Handler, InitializedHandler, UninitHandler};
use messages::{InputMessage, InputMessageBody, OutputMessage, OutputMessageBody};

mod handler;
mod messages;
mod unique_id;

fn main() {
    let initialized_handler = run_uninitialized_loop();
    let Some(mut initialized_handler) = initialized_handler else {
        return;
    };

    let lines = io::stdin().lock().lines();
    for line in lines {
        let line = line.unwrap();
        handle_single_line(&line, &mut initialized_handler);
    }
}

fn run_uninitialized_loop() -> Option<InitializedHandler> {
    let mut stdin = io::stdin().lock();
    let mut handler = UninitHandler::new();
    let mut line = String::new();

    loop {
        let num_read = stdin.read_line(&mut line).unwrap();
        if num_read == 0 {
            break None;
        }

        let line = line.trim();
        handle_single_line(line, &mut handler);
        if let Some(initialized_handler) = handler.into_initialized_handler() {
            break Some(initialized_handler);
        }
    }
}

fn handle_single_line<T: Handler>(line: &str, handler: &mut T) {
    let message: InputMessage = serde_json::from_str(line).unwrap();

    let output_message = handler.handle_message(message).unwrap();

    let serialized =
        serde_json::to_string(&output_message).expect("output message is serializable");
    println!("{}", serialized);
}
