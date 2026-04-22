use crate::builder::PetriNetBuilder;
use crate::petri_net::PetriNet;

use std::collections::{HashSet, VecDeque};

#[derive(Clone, Eq, PartialEq, Hash)]
struct Philosopher {
    id: usize,
    fork_left: usize,
    fork_right: usize,
    state: PhilosopherState,
}

#[derive(Clone, Eq, PartialEq, Hash)]
enum PhilosopherState {
    Thinking,
    Eating,
    InBetween,
}

#[derive(Clone, Eq, PartialEq, Hash)]
struct Fork {
    available: bool,
    id: usize,
}

#[derive(Clone, Eq, PartialEq, Hash)]
struct DiningPhilosophers {
    philosophers: Vec<Philosopher>,
    forks: Vec<Fork>,
}

#[allow(dead_code)]
impl DiningPhilosophers {
    fn new(n: usize) -> Self {
        let philosophers = (0..n).map(|i| Philosopher {
            id: i,
            fork_left: i,
            fork_right: (i + 1) % n,
            state: PhilosopherState::Thinking,
        }).collect();

        let forks = (0..n).map(|i| Fork {
            available: true,
            id: i,
        }).collect();

        DiningPhilosophers { philosophers, forks }
    }

    fn new_with_state_and_forks(states: Vec<Philosopher>, forks: Vec<Fork>) -> Self {
        if states.len() != forks.len() {
            panic!("States and forks vectors must have the same length");
        }

        DiningPhilosophers { philosophers: states, forks }
    }


    fn next_states(&self) -> Vec<DiningPhilosophers> {
        let mut next_states = Vec::new();

        for philosopher in &self.philosophers {
            match philosopher.state {
                PhilosopherState::Thinking => {
                    // l_i: consume left fork and move to the intermediate state.
                    if self.forks[philosopher.fork_left].available {
                        let mut new_state = self.clone();
                        new_state.philosophers[philosopher.id].state = PhilosopherState::InBetween;
                        new_state.forks[philosopher.fork_left].available = false;
                        next_states.push(new_state);
                    }
                }
                PhilosopherState::InBetween => {
                    // r_i: consume right fork and start eating.
                    if self.forks[philosopher.fork_right].available {
                        let mut new_state = self.clone();
                        new_state.philosophers[philosopher.id].state = PhilosopherState::Eating;
                        new_state.forks[philosopher.fork_right].available = false;
                        next_states.push(new_state);
                    }
                }
                PhilosopherState::Eating => {
                    // b_i: finish eating and release both forks.
                    let mut new_state = self.clone();
                    new_state.philosophers[philosopher.id].state = PhilosopherState::Thinking;
                    new_state.forks[philosopher.fork_left].available = true;
                    new_state.forks[philosopher.fork_right].available = true;
                    next_states.push(new_state);
                }
            }
        }

        next_states
    }

}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiningPhilosophersStats {
    pub reachable_count: usize,
    pub deadlock_count: usize,
}


pub fn calculate_dining_philosophers_stats(n: usize) -> DiningPhilosophersStats {
    let initial_state = DiningPhilosophers::new(n);
    let mut visited = HashSet::new();
    let mut to_visit = VecDeque::new();
    let mut deadlock_count = 0usize;

    visited.insert(initial_state.clone());
    to_visit.push_back(initial_state);

    while let Some(current) = to_visit.pop_front() {
        let next_states = current.next_states();
        if next_states.is_empty() {
            deadlock_count += 1;
        }

        for next in next_states {
            if !visited.contains(&next) {
                visited.insert(next.clone());
                to_visit.push_back(next);
            }
        }
    }

    DiningPhilosophersStats {
        reachable_count: visited.len(),
        deadlock_count,
    }
}






pub fn build_dining_philosophers(n: usize) -> PetriNet {
    let mut builder = PetriNetBuilder::new("dining_philosophers".to_string(), "Dining Philosophers".to_string());

    for i in 0..n {
        builder = builder.add_place(format!("thinking{}", i), 1);
        builder = builder.add_place(format!("eating{}", i), 0);
        builder = builder.add_place(format!("inbetween{}", i), 0);
        builder = builder.add_place(format!("fork{}", i), 1);
        builder = builder.add_transition(format!("l{}", i));
        builder = builder.add_transition(format!("r{}", i));
        builder = builder.add_transition(format!("b{}", i));

        let next_i = (i + 1) % n;
        builder = builder.add_arc(format!("b{}", i), format!("fork{}", i));
        builder = builder.add_arc(format!("b{}", i), format!("fork{}", next_i));
        builder = builder.add_arc(format!("fork{}", i), format!("l{}", i));
        builder = builder.add_arc(format!("fork{}", next_i), format!("r{}", i));
        
        builder = builder.add_arc(format!("l{}", i), format!("inbetween{}", i));
        builder = builder.add_arc(format!("inbetween{}", i), format!("r{}", i));
        builder = builder.add_arc(format!("r{}", i), format!("eating{}", i));
        builder = builder.add_arc(format!("eating{}", i), format!("b{}", i));

    }



    builder.build()
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate_dining_philosophers_deadlocks() {
        let n = 4;
        let stats = calculate_dining_philosophers_stats(n);
        assert_eq!(stats.reachable_count, 34);
        assert_eq!(stats.deadlock_count, 1);
    }
}