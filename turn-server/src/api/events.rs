use std::sync::LazyLock;

use axum::response::sse::Event;
use serde::Serialize;
use tokio::sync::broadcast::{Sender, channel};
use tokio_stream::wrappers::BroadcastStream;

static CHANNEL: LazyLock<Sender<Event>> = LazyLock::new(|| channel(10).0);

pub fn get_event_stream() -> BroadcastStream<Event> {
    BroadcastStream::new(CHANNEL.subscribe())
}

pub fn send_with_stream<T, F>(event: &str, handle: F)
where
    F: FnOnce() -> T,
    T: Serialize,
{
    if CHANNEL.receiver_count() > 0 {
        let _ = CHANNEL.send(Event::default().event(event).json_data(handle()).unwrap());
    }
}
