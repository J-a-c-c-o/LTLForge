use std::collections::VecDeque;
use std::collections::HashMap;

use crate::petri_net::{PetriNet, PetriState};

pub struct ReachabilityStats {
    pub reachable_count: usize,
    pub deadlock_count: usize,
}

struct StateInterner {
    by_state: HashMap<PetriState, u32>,
    states: Vec<PetriState>,
}

impl StateInterner {
    fn new() -> Self {
        Self {
            by_state: HashMap::new(),
            states: Vec::new(),
        }
    }

    fn intern(&mut self, state: PetriState) -> (u32, bool) {
        if let Some(&id) = self.by_state.get(&state) {
            return (id, false);
        }

        let id = self.states.len() as u32;
        self.states.push(state.clone());
        self.by_state.insert(state, id);
        (id, true)
    }

    fn get(&self, id: u32) -> &PetriState {
        &self.states[id as usize]
    }
}

pub fn get_reachability_stats(net: &PetriNet) -> ReachabilityStats {
    let mut interner = StateInterner::new();
    let mut to_visit: VecDeque<u32> = VecDeque::new();

    let mut reachable_count = 0usize;
    let mut deadlock_count = 0usize;

    let (initial_id, _) = interner.intern(net.initial_state());
    to_visit.push_back(initial_id);

    while let Some(current_id) = to_visit.pop_front() {
        reachable_count += 1;
        let current_tokens = interner.get(current_id).tokens.clone();
        let current_state = PetriState {
            tokens: current_tokens,
        };
        let mut generated = 0usize;
        for successor in net.next_states(&current_state) {
            let (successor_id, is_new) = interner.intern(successor);
            if is_new {
                to_visit.push_back(successor_id);
            }
            generated += 1;
        }

        if generated == 0 {
            deadlock_count += 1;
        }
    }

    ReachabilityStats {
        reachable_count,
        deadlock_count,
    }
}
