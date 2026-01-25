use crate::sessions::Session;
use crate::{tl, Error};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tl::types::updates::State;
use tokio::time;
use tokio::time::{Instant, Interval, MissedTickBehavior};

#[derive(Clone)]
pub struct Checkpoint {
    pub after: Duration,
    pub changes: u64,
}

impl Checkpoint {
    pub fn new(seconds: u64, operations: u64) -> Self {
        Self {
            after: Duration::from_secs(seconds),
            changes: operations,
        }
    }
}

/// This struct has 3 purposes:
/// 1. Manage states.
/// 2. Dump states to disk based on RDB (Redis Database) algorithm.
/// 3. Store whether states are initialized.
pub struct Updater<S> {
    session: Arc<S>,
    states: HashMap<i64, State>,
    interval: Interval,
    started: Instant,
    operations: u64,
    checkpoints: Vec<Checkpoint>,
    initialized: bool,
}

impl<S: Session> Updater<S> {
    pub fn new(
        session: Arc<S>,
        states: HashMap<i64, State>,
        interval: Duration,
        checkpoints: Vec<Checkpoint>,
    ) -> Self {
        Self {
            session,
            states,
            interval: {
                let mut interval = time::interval(interval);
                interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
                interval
            },
            started: Instant::now(),
            operations: 0,
            checkpoints,
            initialized: false,
        }
    }

    pub fn initialized(&self) -> bool {
        self.initialized
    }

    pub fn set_initialized(&mut self) {
        self.initialized = true;
    }

    pub async fn checkpoint(&mut self) {
        self.interval.tick().await;
    }

    pub fn have_self(&self) -> bool {
        self.have_state(0)
    }

    pub fn have_state(&self, id: i64) -> bool {
        self.states.contains_key(&id)
    }

    pub fn get_self(&self) -> &State {
        self.get_state(0)
            .expect("updater should be initialized")
    }

    pub fn get_state(&self, id: i64) -> Option<&State> {
        self.states.get(&id)
    }

    pub fn set_self<F>(&mut self, f: F)
    where
        F: Fn(&mut State),
    {
        self.set_state(0, f);
    }

    pub fn set_state<F>(&mut self, id: i64, f: F)
    where
        F: Fn(&mut State),
    {
        let state = self.states.entry(id)
            .or_insert(State {
                pts: 0,
                qts: 0,
                date: 0,
                seq: 0,
                unread_count: 0,
            });
        f(state);
        self.operations += 1;
    }

    pub fn need_save(&self) -> bool {
        for checkpoint in &self.checkpoints {
            let now = Instant::now();
            let diff = now - self.started;

            if diff > checkpoint.after && self.operations >= checkpoint.changes {
                return true;
            }
        }

        false
    }

    pub async fn save(&mut self) -> Result<(), Error> {
        self.session.set_states(&self.states).await?;

        self.started = Instant::now();
        self.operations = 0;

        Ok(())
    }
}
