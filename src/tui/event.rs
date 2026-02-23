use std::{
    sync::mpsc,
    thread,
    time::Duration,
};

use crossterm::event::{self, Event, KeyEvent};

pub enum AppEvent {
    Tick,
    Key(KeyEvent),
}

pub struct EventHandler {
    receiver: mpsc::Receiver<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();

        let tx_tick = tx.clone();
        thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    let _ = tx.send(AppEvent::Key(key));
                }
            } else {
                let _ = tx_tick.send(AppEvent::Tick);
            }
        });

        Self { receiver: rx }
    }

    pub fn next(&self) -> Result<AppEvent, mpsc::RecvError> {
        self.receiver.recv()
    }
}