use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, LazyLock},
};

use bytes::BytesMut;
use crossbeam_channel::{Receiver, Sender, unbounded};

pub struct Buffer {
    sender: Arc<Sender<BytesMut>>,
    bytes: Option<BytesMut>,
}

impl Deref for Buffer {
    type Target = BytesMut;

    fn deref(&self) -> &Self::Target {
        self.bytes
            .as_ref()
            .expect("buffer is already returned to the pool")
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.bytes
            .as_mut()
            .expect("buffer is already returned to the pool")
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let _ = self.sender.send(
            self.bytes
                .take()
                .expect("buffer is already returned to the pool"),
        );
    }
}

static MEMORY_POOL: LazyLock<MemoryPool> = LazyLock::new(|| MemoryPool::new());

/// A simple memory pool for reusing buffers. The pool is implemented using a channel, and the buffers are returned to the pool when they are dropped.
pub struct MemoryPool {
    receiver: Receiver<BytesMut>,
    sender: Arc<Sender<BytesMut>>,
}

impl MemoryPool {
    /// The maximum size of a message that can be read from the stream. This is determined by the first 4 bytes of the message, which indicate the size of the message.
    pub const MAX_MESSAGE_SIZE: usize = 4096;

    /// Acquire a buffer from the pool. If the pool is empty, a new buffer will be created with a capacity of `MAX_MESSAGE_SIZE`.
    pub fn acquire() -> Buffer {
        MEMORY_POOL.get_buffer()
    }

    fn new() -> Self {
        let (sender, receiver) = unbounded::<BytesMut>();

        Self {
            sender: Arc::new(sender),
            receiver,
        }
    }

    fn get_buffer(&self) -> Buffer {
        Buffer {
            sender: self.sender.clone(),
            bytes: Some(
                self.receiver
                    .try_recv()
                    .ok()
                    .unwrap_or_else(|| BytesMut::with_capacity(Self::MAX_MESSAGE_SIZE)),
            ),
        }
    }
}
