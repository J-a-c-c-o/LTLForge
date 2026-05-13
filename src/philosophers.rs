use crate::builder::PetriNetBuilder;
use crate::petri_net::PetriNet;

use std::collections::VecDeque;
use rustc_hash::FxHashSet;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
enum Stage {
    Thinking,
    InbetweenLeft,
    InbetweenRight,
    Eating,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Philosopher {
    id: usize,
    stage: Stage,
    fork_left: usize,
    fork_right: usize,
    configuration: PhilosopherConfiguration,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct PhilosopherConfiguration {
    pub allowed_left: bool,
    pub allowed_right: bool,
}

impl Philosopher {
    fn new(id: usize, n: usize, configuration: PhilosopherConfiguration) -> Self {
        Philosopher {
            id,
            stage: Stage::Thinking,
            fork_left: id,
            fork_right: (id + 1) % n,
            configuration,
        }
    }

    fn pick_up_left(&mut self, forks: &mut [bool]) -> bool {
        if self.configuration.allowed_left
            && matches!(self.stage, Stage::Thinking)
            && !forks[self.fork_left]
        {
            forks[self.fork_left] = true;
            self.stage = Stage::InbetweenLeft;
            return true;
        }
        if self.configuration.allowed_right
            && matches!(self.stage, Stage::InbetweenRight)
            && !forks[self.fork_left]
        {
            forks[self.fork_left] = true;
            self.stage = Stage::Eating;
            return true;
        }
        false
    }

    fn pick_up_right(&mut self, forks: &mut [bool]) -> bool {
        if self.configuration.allowed_left
            && matches!(self.stage, Stage::InbetweenLeft)
            && !forks[self.fork_right]
        {
            forks[self.fork_right] = true;
            self.stage = Stage::Eating;
            return true;
        }
        if self.configuration.allowed_right
            && matches!(self.stage, Stage::Thinking)
            && !forks[self.fork_right]
        {
            forks[self.fork_right] = true;
            self.stage = Stage::InbetweenRight;
            return true;
        }
        false
    }

    fn put_down_forks(&mut self, forks: &mut [bool]) -> bool {
        // b transition: from Eating -> Thinking, release both forks
        if !matches!(self.stage, Stage::Eating) {
            return false;
        }

        forks[self.fork_left] = false;
        forks[self.fork_right] = false;
        self.stage = Stage::Thinking;
        true
    }
}

type ReachableState = (Vec<Philosopher>, Vec<bool>);

pub fn compute_reachable_states_and_deadlocks(
    n: usize,
    configurations: Vec<PhilosopherConfiguration>,
) -> (FxHashSet<ReachableState>, FxHashSet<ReachableState>) {
    let philosophers: Vec<Philosopher> = (0..n)
        .map(|i| Philosopher::new(i, n, configurations[i].clone()))
        .collect();
    let forks = vec![false; n];
    let mut states = FxHashSet::default();
    let mut deadlocks = FxHashSet::default();
    let mut queue = VecDeque::new();

    queue.push_back((philosophers.clone(), forks.clone()));

    while let Some((mut philosophers, forks)) = queue.pop_front() {
        if !states.insert((philosophers.clone(), forks.clone())) {
            continue;
        }

        let mut actions = 0;
        for i in 0..n {
            let philosopher = philosophers[i].clone();
            let mut new_forks = forks.clone();

            if philosophers[i].put_down_forks(&mut new_forks) {
                queue.push_back((philosophers.clone(), new_forks));
                philosophers[i] = philosopher.clone();
                actions += 1;
            }

            let mut new_forks = forks.clone();
            if philosophers[i].pick_up_left(&mut new_forks) {
                queue.push_back((philosophers.clone(), new_forks));
                philosophers[i] = philosopher.clone();
                actions += 1;
            }

            let mut new_forks = forks.clone();
            if philosophers[i].pick_up_right(&mut new_forks) {
                queue.push_back((philosophers.clone(), new_forks));
                philosophers[i] = philosopher.clone();
                actions += 1;
            }
        }

        if actions == 0 {
            deadlocks.insert((philosophers.clone(), forks.clone()));
        }
    }

    (states, deadlocks)
}

pub fn build_dining_philosophers(
    n: usize,
    configurations: Vec<PhilosopherConfiguration>,
) -> PetriNet {
    let mut builder = PetriNetBuilder::new(
        "dining_philosophers".to_string(),
        "Dining Philosophers".to_string(),
    );

    for (i, config) in configurations.iter().enumerate().take(n) {
        let next_i = (i + 1) % n;

        builder = builder.add_place(format!("thinking{}", i), 1);
        builder = builder.add_place(format!("eating{}", i), 0);
        builder = builder.add_place(format!("inbetweenL{}", i), 0);
        builder = builder.add_place(format!("inbetweenR{}", i), 0);
        builder = builder.add_place(format!("fork{}", i), 1);

        builder = builder.add_transition(format!("b{}", i));
        builder = builder.add_arc(format!("eating{}", i), format!("b{}", i));
        builder = builder.add_arc(format!("b{}", i), format!("thinking{}", i));
        builder = builder.add_arc(format!("b{}", i), format!("fork{}", i));
        builder = builder.add_arc(format!("b{}", i), format!("fork{}", next_i));

        if config.allowed_left {
            builder = builder.add_transition(format!("l{}_start", i));
            builder = builder.add_transition(format!("l{}_finish", i));

            builder = builder.add_arc(format!("thinking{}", i), format!("l{}_start", i));
            builder = builder.add_arc(format!("fork{}", i), format!("l{}_start", i));
            builder = builder.add_arc(format!("l{}_start", i), format!("inbetweenL{}", i));

            builder = builder.add_arc(format!("inbetweenL{}", i), format!("l{}_finish", i));
            builder = builder.add_arc(format!("fork{}", next_i), format!("l{}_finish", i));
            builder = builder.add_arc(format!("l{}_finish", i), format!("eating{}", i));
        }

        if config.allowed_right {
            builder = builder.add_transition(format!("r{}_start", i));
            builder = builder.add_transition(format!("r{}_finish", i));

            builder = builder.add_arc(format!("thinking{}", i), format!("r{}_start", i));
            builder = builder.add_arc(format!("fork{}", next_i), format!("r{}_start", i));
            builder = builder.add_arc(format!("r{}_start", i), format!("inbetweenR{}", i));

            builder = builder.add_arc(format!("inbetweenR{}", i), format!("r{}_finish", i));
            builder = builder.add_arc(format!("fork{}", i), format!("r{}_finish", i));
            builder = builder.add_arc(format!("r{}_finish", i), format!("eating{}", i));
        }
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explorer::get_reachability_stats;

    #[test]
    fn test_calculate_dining_philosophers_both() {
        let n = 4;
        let configurations = vec![
            PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: true,
            },
            PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: true,
            },
            PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: true,
            },
            PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: true,
            },
        ];

        let (states, deadlocks) = compute_reachable_states_and_deadlocks(n, configurations);
        assert_eq!(states.len(), 81);
        assert_eq!(deadlocks.len(), 2);
    }

    #[test]
    fn test_calculate_dining_philosophers_left_only() {
        let n = 4;
        let configurations = vec![
            PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: false,
            },
            PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: false,
            },
            PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: false,
            },
            PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: false,
            },
        ];

        let (states, deadlocks) = compute_reachable_states_and_deadlocks(n, configurations);
        assert_eq!(states.len(), 34);
        assert_eq!(deadlocks.len(), 1);
    }

    #[test]
    fn test_calculate_dining_philosophers_one_left_rest_right() {
        let n = 4;
        let configurations = vec![
            PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: false,
            },
            PhilosopherConfiguration {
                allowed_left: false,
                allowed_right: true,
            },
            PhilosopherConfiguration {
                allowed_left: false,
                allowed_right: true,
            },
            PhilosopherConfiguration {
                allowed_left: false,
                allowed_right: true,
            },
        ];

        let (states, deadlocks) = compute_reachable_states_and_deadlocks(n, configurations);
        assert_eq!(states.len(), 29);
        assert_eq!(deadlocks.len(), 0);
    }

    #[test]
    fn test_build_dining_philosophers_matches() {
        let cases = vec![
            vec![
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: true,
                },
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: true,
                },
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: true,
                },
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: true,
                },
            ],
            vec![
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: false,
                },
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: false,
                },
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: false,
                },
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: false,
                },
            ],
            vec![
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: false,
                },
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: true,
                },
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: true,
                },
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: false,
                },
            ],
            vec![
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: false,
                },
                PhilosopherConfiguration {
                    allowed_left: false,
                    allowed_right: true,
                },
                PhilosopherConfiguration {
                    allowed_left: false,
                    allowed_right: true,
                },
                PhilosopherConfiguration {
                    allowed_left: false,
                    allowed_right: true,
                },
            ],
        ];

        for configurations in cases {
            let n = configurations.len();
            let (states, deadlocks) =
                compute_reachable_states_and_deadlocks(n, configurations.clone());
            let petri_net = build_dining_philosophers(n, configurations);
            let stats = get_reachability_stats(&petri_net);

            assert_eq!(stats.reachable_count, states.len());
            assert_eq!(stats.deadlock_count, deadlocks.len());
        }
    }
}
