use std::{
    sync::{atomic::AtomicUsize, Arc, RwLock},
    time::Duration,
};

use tokio::{sync::mpsc::UnboundedReceiver, time::sleep};
use turn_driver::controller::{Controller, Session, Stats, Users};

use crate::{events::Events, util::EasyAtomic};

#[derive(Default)]
pub struct State {
    stats: RwLock<Arc<Stats>>,
    users: RwLock<Arc<Users>>,
    previous_users: RwLock<Arc<Users>>,
    session: RwLock<Arc<Option<Session>>>,
}

impl State {
    pub fn get_stats(&self) -> Arc<Stats> {
        self.stats.read().unwrap().clone()
    }

    pub fn get_users(&self) -> Arc<Users> {
        self.users.read().unwrap().clone()
    }

    pub fn get_previous_users(&self) -> Arc<Users> {
        self.previous_users.read().unwrap().clone()
    }

    pub fn get_session(&self) -> Arc<Option<Session>> {
        self.session.read().unwrap().clone()
    }
}

#[derive(Default)]
struct Context {
    get_users_skip: AtomicUsize,
}

pub async fn create_state(
    rpc: Arc<Controller>,
    mut receiver: UnboundedReceiver<Events>,
) -> Arc<State> {
    let state = Arc::new(State::default());
    let ctx = Arc::new(Context::default());

    let rpc_ = rpc.clone();
    let ctx_ = ctx.clone();
    let state_ = state.clone();
    tokio::spawn(async move {
        {
            *state_.stats.write().unwrap() = Arc::new(rpc_.get_stats().await?);
        }

        while let Some(event) = receiver.recv().await {
            match event {
                Events::GetUsers(skip) => {
                    ctx_.get_users_skip.set(skip as usize);
                }
                Events::GetSession(addr) => {
                    *state_.session.write().unwrap() = Arc::new(rpc_.get_session(&addr).await?)
                }
                Events::ClearSession => {
                    *state_.session.write().unwrap() = Arc::new(None);
                }
                Events::RemoveSession(addr) => {
                    rpc_.remove_session(&addr).await?;
                }
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    let state_ = state.clone();
    tokio::spawn(async move {
        loop {
            {
                *state_.previous_users.write().unwrap() = state_.users.read().unwrap().clone();
            }

            *state_.users.write().unwrap() = Arc::new(
                rpc.get_users(ctx.get_users_skip.get() as u32 * 20, 20)
                    .await?,
            );

            sleep(Duration::from_secs(5)).await;
        }

        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    });

    state
}
