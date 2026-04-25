use std::{
    ops::{Deref, DerefMut},
    sync::{
        Arc, LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use bytes::BytesMut;
use crossbeam_channel::{Receiver, Sender, unbounded};
use tokio::time::interval;

static MEMORY_POOL: LazyLock<MemoryPool> = LazyLock::new(|| MemoryPool::new());

// The number of buffers that are currently acquired from the pool. This is used
// for monitoring the usage of the pool.
static ACQUIRE_BUFFER_NUM: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(0));

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
        // A recycled buffer must be logically empty before it is returned.
        // We intentionally keep the allocated capacity to avoid frequent
        // re-allocation on the next acquire.
        if let Some(bytes) = self.bytes.as_mut() {
            bytes.clear();
        }

        // The pool is intentionally unbounded. Returning to the channel is a
        // fast path, and the periodic shrink task is responsible for trimming
        // idle inventory when traffic stays low.
        let _ = self.sender.send(
            self.bytes
                .take()
                .expect("buffer is already returned to the pool"),
        );

        ACQUIRE_BUFFER_NUM.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A simple memory pool for reusing buffers. The pool is implemented using a
/// channel, and the buffers are returned to the pool when they are dropped.
pub struct MemoryPool {
    receiver: Arc<Receiver<BytesMut>>,
    sender: Arc<Sender<BytesMut>>,
}

impl MemoryPool {
    /// The maximum size of a message that can be read from the stream. This is
    /// determined by the first 4 bytes of the message, which indicate the size
    /// of the message.
    pub const MAX_MESSAGE_SIZE: usize = 4096;

    /// Acquire a buffer from the pool. If the pool is empty, a new buffer will
    /// be created with a capacity of `MAX_MESSAGE_SIZE`.
    pub fn acquire() -> Buffer {
        MEMORY_POOL.get_buffer()
    }

    fn new() -> Self {
        let (sender, receiver) = unbounded::<BytesMut>();
        let receiver = Arc::new(receiver);

        // A background cleanup thread that periodically scans and, based on
        // conditions, gradually shrinks the buffer.
        {
            let receiver_ = receiver.clone();

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(10));

                let mut continuous_decline = false;
                let mut tick_steps = 0;

                loop {
                    interval.tick().await;

                    let acquire_size = ACQUIRE_BUFFER_NUM.load(Ordering::SeqCst);
                    let buffer_size = receiver_.len();

                    // If the number of idle buffers is more than 3 times the
                    //  number of acquired buffers, we consider it as a potential
                    // memory leak and start to track the decline of idle buffers.
                    //
                    // If the decline continues for 1 minute (6 ticks), we will
                    // try to shrink the pool by dropping some idle buffers.
                    if buffer_size >= acquire_size * 3 {
                        if tick_steps == 0 {
                            continuous_decline = true;
                        }
                    } else {
                        continuous_decline = false;
                    }

                    tick_steps += 1;

                    if tick_steps >= 6 {
                        tick_steps = 0;

                        if continuous_decline {
                            continuous_decline = false;

                            // The shrink strategy is simple: if the number of
                            // idle buffers is more than 3, we will drop 2/3 of
                            // the idle buffers; otherwise, we will drop all
                            // idle buffers.
                            for _ in 0..if buffer_size <= 3 {
                                buffer_size
                            } else {
                                buffer_size / 3
                            } {
                                if receiver_.try_recv().is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            });
        }

        Self {
            sender: Arc::new(sender),
            receiver,
        }
    }

    fn get_buffer(&self) -> Buffer {
        ACQUIRE_BUFFER_NUM.fetch_add(1, Ordering::SeqCst);

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
