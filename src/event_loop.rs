use std::{
    io::{self, BufRead},
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::{
    handler::{Handler, InitializedHandler},
    messages::Message,
};

enum Event {
    Message(String),
    Timer,
}

const CHANNEL_SIZE: usize = 10;

pub fn handle_single_line<T: Handler>(line: &str, handler: &mut T) {
    let message: Message = serde_json::from_str(line).unwrap();

    if let Err(e) = handler.handle_message(message) {
        eprintln!("failed to handle message: {e:?}");
    }
}

///
/// We run 3 threads
///     1. Reading from stdin
///     2. Timer
///     3. Handler that reacts to both other threads
///
/// We use threads instead of async because I want to learn how to use threads this time :)
pub fn run_initialized_loop(initialized_handler: InitializedHandler) {
    let (events_tx, events_rx) = mpsc::sync_channel(CHANNEL_SIZE);

    let stdin_tx = events_tx.clone();

    std::thread::spawn(move || {
        let lines = io::stdin().lock().lines();
        for line in lines {
            let line = line.unwrap();
            if let Err(e) = stdin_tx.send(Event::Message(line)) {
                eprintln!("failed to new line message: {e:?}")
            }
        }
    });

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(150));
        if let Err(e) = events_tx.send(Event::Timer) {
            eprintln!("failed to send new timer event: {e:?}");
        }
    });

    run_handler_forever(initialized_handler, events_rx);
}

fn run_handler_forever(mut initialized_handler: InitializedHandler, rx: mpsc::Receiver<Event>) {
    loop {
        let event = match rx.recv() {
            Ok(event) => event,
            Err(e) => {
                eprintln!("handler recv error: {e:?}");
                break;
            }
        };

        match event {
            Event::Message(line) => handle_single_line(&line, &mut initialized_handler),
            Event::Timer => initialized_handler.handle_gossip_timer(),
        }
    }
}
