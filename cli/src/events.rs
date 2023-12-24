use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub enum Events {
    GetUsers(u32),
    GetSession(String),
    ClearSession,
}

#[derive(Clone)]
pub struct EventProxy {
    sender: UnboundedSender<Events>,
}

impl EventProxy {
    pub fn new() -> (Self, UnboundedReceiver<Events>) {
        let (sender, receiver) = unbounded_channel();
        (Self { sender }, receiver)
    }

    pub fn get_sender(&self) -> EventSender {
        EventSender {
            sender: self.sender.clone(),
        }
    }
}

pub struct EventSender {
    sender: UnboundedSender<Events>,
}

impl EventSender {
    pub fn send(&self, event: Events) {
        let _ = self.sender.send(event);
    }
}
