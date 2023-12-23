use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc, RwLock,
    },
    time::Duration,
};

use tokio::{sync::mpsc::UnboundedReceiver, time::sleep};

use crate::{
    events::Events,
    rpc::{
        proto::{Report, Stats},
        Rpc,
    },
    util::EasyAtomic,
};

#[derive(Default)]
pub struct State {
    report: RwLock<Arc<Vec<Report>>>,
    stats: RwLock<Arc<Stats>>,
}

impl State {
    pub fn get_stats(&self) -> Arc<Stats> {
        self.stats.read().unwrap().clone()
    }

    pub fn get_report(&self) -> Arc<Vec<Report>> {
        self.report.read().unwrap().clone()
    }
}

#[derive(Default)]
struct Context {
    get_report: AtomicBool,
    get_report_skip: AtomicUsize,
}

pub async fn create_state(rpc: Arc<Rpc>, mut receiver: UnboundedReceiver<Events>) -> Arc<State> {
    let state = Arc::new(State::default());
    let ctx = Arc::new(Context::default());

    let rpc_ = rpc.clone();
    let ctx_ = ctx.clone();
    tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            match event {
                Events::GetReport(skip) => {
                    ctx_.get_report_skip.set(skip as usize);
                }
                Events::StartGetReport => {
                    ctx_.get_report.set(true);
                }
                Events::StopGetReport => {
                    ctx_.get_report.set(false);
                }
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    let state_ = state.clone();
    tokio::spawn(async move {
        loop {
            {
                *state_.stats.write().unwrap() = Arc::new(rpc_.get_status().await?);
            }

            if ctx.get_report.get() {
                *state_.report.write().unwrap() = Arc::new(
                    rpc.get_report(ctx.get_report_skip.get() as u32 * 20, 20)
                        .await?
                        .reports,
                );
            }

            sleep(Duration::from_secs(5)).await;
        }

        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    });

    state
}
