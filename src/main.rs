use std::io::{self, BufRead};

use event_loop::{handle_single_line, run_initialized_loop};
use handler::{Handler, InitializedHandler, UninitHandler};
use messages::Message;

mod event_loop;
mod handler;
mod messages;
mod unique_id;

fn main() {
    // wait for handler initialization message
    let initialized_handler = run_uninitialized_loop();
    let Some(initialized_handler) = initialized_handler else {
        return;
    };

    // After the handler is initialized, run it to process all new messages
    run_initialized_loop(initialized_handler);
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
        if let Some(initialized_handler) = handler.get_initialized_handler() {
            break Some(initialized_handler);
        }
    }
}
