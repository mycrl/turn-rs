use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use clap::Parser;

#[rustfmt::skip]
pub static SOFTWARE: &str = concat!(
    "turn manager - ", 
    env!("CARGO_PKG_VERSION")
);

#[derive(Parser)]
#[command(
    about = env!("CARGO_PKG_DESCRIPTION"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
pub struct Opts {
    #[arg(long)]
    pub uri: String,
}

pub trait EasyAtomic {
    type Item;

    fn get(&self) -> Self::Item;
    fn set(&self, value: Self::Item);
}

impl EasyAtomic for AtomicUsize {
    type Item = usize;

    fn get(&self) -> Self::Item {
        self.load(Ordering::Relaxed)
    }

    fn set(&self, value: Self::Item) {
        self.store(value, Ordering::Relaxed);
    }
}

impl EasyAtomic for AtomicBool {
    type Item = bool;

    fn get(&self) -> Self::Item {
        self.load(Ordering::Relaxed)
    }

    fn set(&self, value: Self::Item) {
        self.store(value, Ordering::Relaxed);
    }
}
