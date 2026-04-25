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
use tokio::time::sleep;

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

/// Pool observe interval in seconds.
const OBSERVE_INTERVAL_SECS: u64 = 10;

/// Start shrinking only after about 1 minute of continuous low usage.
const LOW_USAGE_WINDOW_TICKS: u32 = 6;

/// Do not shrink below this idle size so burst traffic can still be
/// absorbed quickly.
const MIN_IDLE_KEEP: usize = 256;

/// Low usage condition: acquired <= idle / 4.
const LOW_USAGE_RATIO_NUM: usize = 1;
const LOW_USAGE_RATIO_DEN: usize = 4;

/// Cooldown between two shrink operations to avoid overly aggressive reclaim.
const SHRINK_COOLDOWN_TICKS: u32 = 2;

/// Stair-step shrink parameters.
const SHRINK_DIVISOR: usize = 4;
const SHRINK_MIN_STEP: usize = 32;
const SHRINK_MAX_STEP: usize = 512;

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

        {
            let receiver_ = receiver.clone();

            tokio::spawn(async move {
                // Number of consecutive observe ticks that satisfy "low usage".
                // Once this reaches LOW_USAGE_WINDOW_TICKS (~1 minute), we
                // start reclaiming idle buffers in steps.
                let mut low_usage_ticks = 0u32;

                // Cooldown prevents shrinking on every tick. This keeps the
                // curve smooth and avoids over-reacting to temporary dips.
                let mut shrink_cooldown = 0u32;

                loop {
                    sleep(Duration::from_secs(OBSERVE_INTERVAL_SECS)).await;

                    let acquire_size = ACQUIRE_BUFFER_NUM.load(Ordering::SeqCst);
                    let buffer_size = receiver_.len();

                    // Low usage means: a large idle inventory exists while the
                    // in-flight demand stays relatively small (<= idle / 4).
                    // We also require idle > MIN_IDLE_KEEP so the pool always
                    // keeps a burst-friendly baseline.
                    let low_usage = buffer_size > MIN_IDLE_KEEP
                        && acquire_size.saturating_mul(LOW_USAGE_RATIO_DEN)
                            <= buffer_size.saturating_mul(LOW_USAGE_RATIO_NUM);

                    if low_usage {
                        low_usage_ticks = low_usage_ticks.saturating_add(1);
                    } else {
                        low_usage_ticks = 0;
                        shrink_cooldown = 0;
                    }

                    if shrink_cooldown > 0 {
                        shrink_cooldown -= 1;
                    }

                    // Reclaim only after sustained low usage and cooldown passed.
                    // This is the key to "step-down" behavior instead of a sudden
                    // aggressive drop that could hurt the next traffic burst.
                    if low_usage_ticks >= LOW_USAGE_WINDOW_TICKS && shrink_cooldown == 0 {
                        let excess = buffer_size.saturating_sub(MIN_IDLE_KEEP);
                        if excess > 0 {
                            // Shrink proportionally to excess inventory, but clamp
                            // with min/max bounds so each reclaim round is bounded.
                            let step = (excess / SHRINK_DIVISOR)
                                .clamp(SHRINK_MIN_STEP, SHRINK_MAX_STEP)
                                .min(excess);

                            // Drain from the idle channel and drop immediately,
                            // releasing these buffers back to the allocator.
                            let mut reclaimed = 0usize;
                            for _ in 0..step {
                                if receiver_.try_recv().is_ok() {
                                    reclaimed += 1;
                                } else {
                                    break;
                                }
                            }

                            if reclaimed > 0 {
                                log::debug!(
                                    "memory pool shrink: reclaimed={}, idle_before={}, idle_after={}, acquired={}",
                                    reclaimed,
                                    buffer_size,
                                    buffer_size.saturating_sub(reclaimed),
                                    acquire_size,
                                );
                            }

                            // Enter cooldown before the next reclaim cycle.
                            shrink_cooldown = SHRINK_COOLDOWN_TICKS;
                        }
                    }

                    // Warning signal: live demand is notably larger than idle
                    // inventory. This helps detect burst pressure and tune pool
                    // thresholds if needed.
                    if acquire_size > buffer_size * 2 {
                        log::debug!(
                            "memory pool is running low: acquire_size={}, buffer_size={buffer_size}",
                            acquire_size,
                        );
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
