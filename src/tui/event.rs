use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
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

        thread::spawn(move || {
            let mut last_tick = Instant::now();

            loop {
                // Time remaining until next tick
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                // Poll for input, but only until next tick deadline
                if event::poll(timeout).unwrap_or(false) {
                    if let Ok(event) = event::read() {
                        match event {
                            Event::Key(key) => {
                                let _ = tx.send(AppEvent::Key(key));
                            }
                            _ => {}
                        }
                    }
                }

                // Send Tick if enough time passed
                if last_tick.elapsed() >= tick_rate {
                    let _ = tx.send(AppEvent::Tick);
                    last_tick = Instant::now();
                }
            }
        });

        Self { receiver: rx }
    }

    pub fn next(&self) -> Result<AppEvent, mpsc::RecvError> {
        self.receiver.recv()
    }
}