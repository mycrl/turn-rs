use std::{
    ops::{Deref, DerefMut},
    sync::{
        Arc, LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use bytes::BytesMut;
use crossbeam_queue::ArrayQueue;
use tokio::time::interval;

// The memory pool for reusing buffers.
static MEMORY_POOL: LazyLock<MemoryPool> = LazyLock::new(|| MemoryPool::new());

// The number of buffers that are currently acquired from the pool.
static ACQUIRE_BUFFER_NUM: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(0));

// The memory pool for reusing buffers.
struct MemoryPool(Arc<ArrayQueue<BytesMut>>);

impl MemoryPool {
    // The maximum size of a message that can be read from the stream.
    const MAX_MESSAGE_SIZE: usize = 4096;

    // The maximum size of the queue.
    const MAX_QUEUE_SIZE: usize = 4096;

    // Create a new memory pool.
    fn new() -> Self {
        let queue: Arc<ArrayQueue<BytesMut>> = ArrayQueue::new(Self::MAX_QUEUE_SIZE).into();

        // A background cleanup thread that periodically scans and, based on
        // conditions, gradually shrinks the buffer.
        {
            let queue = queue.clone();

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(10));

                let mut continuous_decline = false;
                let mut tick_steps = 0;

                loop {
                    interval.tick().await;

                    let acquire_size = ACQUIRE_BUFFER_NUM.load(Ordering::Relaxed);
                    let buffer_size = queue.len();

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
                                let _ = queue.pop();
                            }
                        }
                    }
                }
            });
        }

        Self(queue)
    }

    // Acquire a buffer from the pool. If the pool is empty, a new buffer will
    // be created with a capacity of `MAX_MESSAGE_SIZE`.
    fn get_buffer(&self, capacity: Option<usize>) -> Buffer {
        ACQUIRE_BUFFER_NUM.fetch_add(1, Ordering::Relaxed);

        // If the capacity is not provided, use the default capacity.
        // If the capacity is greater than the maximum allowed size, use the
        // maximum allowed size.
        // Otherwise, use the provided capacity.
        let capacity = capacity
            .unwrap_or(Self::MAX_MESSAGE_SIZE)
            .min(Self::MAX_MESSAGE_SIZE);

        Buffer(Some(
            self.0
                .pop()
                .unwrap_or_else(|| BytesMut::with_capacity(capacity)),
        ))
    }

    // Return a buffer to the pool.
    fn return_buffer(&self, buffer: &mut Buffer) {
        if let Some(mut bytes) = buffer.0.take() {
            ACQUIRE_BUFFER_NUM.fetch_sub(1, Ordering::Relaxed);

            // Clear the buffer to reuse it.
            bytes.clear();

            // Push the buffer back to the pool.
            let _ = self.0.push(bytes);
        }
    }
}

pub struct Buffer(Option<BytesMut>);

impl Buffer {
    // The maximum size of a message that can be read from the stream.
    pub const MAX_MESSAGE_SIZE: usize = MemoryPool::MAX_MESSAGE_SIZE;

    /// Acquire a buffer from the pool.
    ///
    /// The capacity of the buffer will be determined by the maximum allowed size.
    /// If the capacity is not provided, the default capacity will be used.
    pub fn new() -> Self {
        MEMORY_POOL.get_buffer(None)
    }

    /// Acquire a buffer from the pool with a specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        MEMORY_POOL.get_buffer(Some(capacity))
    }
}

impl Deref for Buffer {
    type Target = BytesMut;

    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .expect("buffer is already returned to the pool")
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
            .as_mut()
            .expect("buffer is already returned to the pool")
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        MEMORY_POOL.return_buffer(self)
    }
}
