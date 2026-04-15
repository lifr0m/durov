use crate::client::updates::queue::Queue;
use crate::tl;
use std::collections::{HashMap, HashSet};
use tl::types::updates::State;

pub struct Updater {
    states: Option<HashMap<i64, State>>,
    pub bot: bool,
    pub seq_queue: Queue<tl::types::UpdatesCombined>,
    pub pts_queue: Queue<tl::enums::Update>,
    pub qts_queue: Queue<tl::enums::Update>,
    pub channel_queues: HashMap<i64, Queue<tl::enums::Update>>,
    pub recovering: HashSet<i64>,
}

impl Default for Updater {
    fn default() -> Self {
        Self::new()
    }
}

impl Updater {
    pub fn new() -> Self {
        Self {
            states: None,
            bot: false,
            seq_queue: Queue::new(),
            pts_queue: Queue::new(),
            qts_queue: Queue::new(),
            channel_queues: HashMap::new(),
            recovering: HashSet::new(),
        }
    }

    pub fn initialized(&self) -> bool {
        self.states.is_some()
    }

    pub fn initialize(&mut self, states: HashMap<i64, State>) {
        self.states = Some(states);
    }

    pub fn states(&self) -> &HashMap<i64, State> {
        self.states.as_ref()
            .expect("updater should be initialized")
    }

    pub fn have_state(&self, id: i64) -> bool {
        self.states.as_ref()
            .expect("updater should be initialized")
            .contains_key(&id)
    }

    pub fn get_state(&self, id: i64) -> &State {
        self.states.as_ref()
            .expect("updater should be initialized")
            .get(&id)
            .unwrap()
    }

    pub fn set_state<F>(&mut self, id: i64, f: F)
    where
        F: Fn(&mut State),
    {
        let state = self.states.as_mut()
            .expect("updater should be initialized")
            .entry(id)
            .or_insert(State {
                pts: 0,
                qts: 0,
                date: 0,
                seq: 0,
                unread_count: 0,
            });

        f(state);
    }

    pub fn reset_channels(&mut self) {
        self.states.as_mut()
            .expect("updater should be initialized")
            .retain(|&id, _| id == 0);
    }
}
