use std::{
    sync::{atomic::AtomicUsize, Arc, RwLock},
    time::Duration,
};

use tokio::{sync::mpsc::UnboundedReceiver, time::sleep};

use crate::{
    events::Events,
    rpc::{
        proto::{Session, Stats, User},
        Rpc,
    },
    util::EasyAtomic,
};

#[derive(Default)]
pub struct State {
    stats: RwLock<Arc<Stats>>,
    users: RwLock<Arc<Vec<User>>>,
    previous_users: RwLock<Arc<Vec<User>>>,
    session: RwLock<Arc<Option<Session>>>,
}

impl State {
    pub fn get_stats(&self) -> Arc<Stats> {
        self.stats.read().unwrap().clone()
    }

    pub fn get_users(&self) -> Arc<Vec<User>> {
        self.users.read().unwrap().clone()
    }

    pub fn get_previous_users(&self) -> Arc<Vec<User>> {
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

pub async fn create_state(rpc: Arc<Rpc>, mut receiver: UnboundedReceiver<Events>) -> Arc<State> {
    let state = Arc::new(State::default());
    let ctx = Arc::new(Context::default());

    let rpc_ = rpc.clone();
    let ctx_ = ctx.clone();
    let state_ = state.clone();
    tokio::spawn(async move {
        {
            *state_.stats.write().unwrap() = Arc::new(rpc_.get_status().await?);
        }

        while let Some(event) = receiver.recv().await {
            match event {
                Events::GetUsers(skip) => {
                    ctx_.get_users_skip.set(skip as usize);
                }
                Events::GetSession(addr) => {
                    *state_.session.write().unwrap() =
                        Arc::new(rpc_.get_session(addr).await?.session)
                }
                Events::ClearSession => {
                    *state_.session.write().unwrap() = Arc::new(None);
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
                    .await?
                    .users,
            );

            sleep(Duration::from_secs(5)).await;
        }

        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    });

    state
}
