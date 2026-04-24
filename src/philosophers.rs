use crate::builder::PetriNetBuilder;
use crate::petri_net::PetriNet;

use std::collections::{HashSet, VecDeque};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
enum Stage {
    Thinking,
    Inbetween_Left,
    Inbetween_Right,
    Eating,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct Philosopher {
    id: usize,
    stage: Stage,
    fork_left: usize,
    fork_right: usize,
    configuration: PhilosopherConfiguration,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct PhilosopherConfiguration {
    allowed_left: bool,
    allowed_right: bool,
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
        if self.configuration.allowed_left && matches!(self.stage, Stage::Thinking) && !forks[self.fork_left] {
            forks[self.fork_left] = true;
            self.stage = Stage::Inbetween_Left;
            return true;
        }
        if self.configuration.allowed_right && matches!(self.stage, Stage::Inbetween_Right) && !forks[self.fork_left] {
            forks[self.fork_left] = true;
            self.stage = Stage::Eating;
            return true;
        }
        false
    }

    fn pick_up_right(&mut self, forks: &mut [bool]) -> bool {
        if self.configuration.allowed_left && matches!(self.stage, Stage::Inbetween_Left) && !forks[self.fork_right] {
            forks[self.fork_right] = true;
            self.stage = Stage::Eating;
            return true;
        }
        if self.configuration.allowed_right && matches!(self.stage, Stage::Thinking) && !forks[self.fork_right] {
            forks[self.fork_right] = true;
            self.stage = Stage::Inbetween_Right;
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

pub fn play_dining_philosophers(n: usize, configurations: Vec<PhilosopherConfiguration>) -> (HashSet<(Vec<Philosopher>, Vec<bool>)>, HashSet<(Vec<Philosopher>, Vec<bool>)>) {
    let mut philosophers: Vec<Philosopher> = (0..n).map(|i| Philosopher::new(i, n, configurations[i].clone())).collect();
    let mut forks = vec![false; n];
    let mut states = HashSet::new();
    let mut deadlocks = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back((philosophers.clone(), forks.clone()));

    while let Some((mut philosophers, mut forks)) = queue.pop_front() {
        if !states.insert((philosophers.clone(), forks.clone())) {
            continue;
        }

        let mut actions = 0;
        for i in 0..n {
            let mut philosopher = philosophers[i].clone();
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
    fn test_calculate_dining_philosophers_both() {
        let n = 4;
        let configurations = vec![
            PhilosopherConfiguration { allowed_left: true, allowed_right: true },
            PhilosopherConfiguration { allowed_left: true, allowed_right: true },
            PhilosopherConfiguration { allowed_left: true, allowed_right: true },
            PhilosopherConfiguration { allowed_left: true, allowed_right: true }
        ];


        let (states, deadlocks) = play_dining_philosophers(n, configurations);
        assert_eq!(states.len(), 81);
        assert_eq!(deadlocks.len(), 2);
    }

    #[test]
    fn test_calculate_dining_philosophers_left_only() {
        let n = 4;
        let configurations = vec![
            PhilosopherConfiguration { allowed_left: true, allowed_right: false },
            PhilosopherConfiguration { allowed_left: true, allowed_right: false },
            PhilosopherConfiguration { allowed_left: true, allowed_right: false },
            PhilosopherConfiguration { allowed_left: true, allowed_right: false }
        ];

        let (states, deadlocks) = play_dining_philosophers(n, configurations);
        assert_eq!(states.len(), 34);
        assert_eq!(deadlocks.len(), 1);
    }
}
