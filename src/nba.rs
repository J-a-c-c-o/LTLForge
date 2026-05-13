use crate::gnba::GNBA;
use crate::ltl_parser::LTL;
use std::collections::VecDeque;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct NBA {
    pub closure: Vec<LTL>,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub initial_states: Vec<usize>,
    pub acceptance_condition: AcceptanceCondition,
    new_to_gnba: Vec<(usize, usize, usize)>,
    gnba_to_new: FxHashMap<(usize, usize), usize>,
    gnba: GNBA,
}

pub struct State {
    pub id: usize,
    pub formulas: Vec<bool>,
}

pub struct Transition {
    pub from: usize,
    pub to: usize,
    pub label: Vec<bool>,
}

pub struct AcceptanceCondition {
    pub id: usize,
    pub states: Vec<usize>,
}

impl NBA {
    pub fn new(ltl: &LTL) -> Self {
        let gnba = GNBA::new(ltl);

        // Create x copies of each accepting state, where x is the number of acceptance conditions
        let mut states = Vec::new();
        let mut new_to_gnba = Vec::new();
        let mut gnba_to_new = FxHashMap::default();
        let mut new_state_id = 0;
        let closure = gnba.closure.clone();

        for state in &gnba.states {
            for acc_id in 0..gnba.acceptance_conditions.len() {
                states.push(State {
                    id: new_state_id,
                    formulas: state.formulas.clone(),
                });
                new_to_gnba.push((state.id, acc_id, new_state_id));
                gnba_to_new.insert((state.id, acc_id), new_state_id);
                new_state_id += 1;
            }
        }

        let initial_states: Vec<usize> = gnba
            .initial_states
            .iter()
            .map(|orig_id| {
                new_to_gnba
                    .iter()
                    .find(|(orig_id_map, acc, _)| *orig_id_map == *orig_id && *acc == 0)
                    .unwrap()
                    .2
            })
            .collect();

        // Accepting state: first of the gnba
        let acceptance_condition = AcceptanceCondition {
            id: 0,
            states: new_to_gnba
                .iter()
                .filter_map(|(orig_id, acc, new_id)| {
                    if gnba.acceptance_conditions[0].states.contains(orig_id) && *acc == 0 {
                        Some(*new_id)
                    } else {
                        None
                    }
                })
                .collect(),
        };
        let mut nba = NBA {
            closure,
            states,
            transitions: Vec::new(),
            initial_states,
            acceptance_condition,
            new_to_gnba,
            gnba_to_new,
            gnba,
        };
        nba.remove_dead_states();
        nba
    }

    pub fn initial_states(&self) -> Vec<usize> {
        self.initial_states.clone()
    }

    pub fn is_accepting(&self, state_id: usize) -> bool {
        self.acceptance_condition.states.contains(&state_id)
    }

    pub fn next(&self, state_id: usize, label: &[bool]) -> Vec<usize> {
        let Some(&(orig_state_id, _acc_id, _)) = self.new_to_gnba.get(state_id) else {
            return Vec::new();
        };
        if self.gnba_state_label(orig_state_id).as_slice() != label {
            return Vec::new();
        }

        self.successors(state_id)
    }

    pub fn successors(&self, state_id: usize) -> Vec<usize> {
        self.transitions
            .iter()
            .filter_map(|transition| {
                if transition.from == state_id {
                    Some(transition.to)
                } else {
                    None
                }
            })
            .collect()
    }

    fn gnba_state_label(&self, gnba_state_id: usize) -> Vec<bool> {
        self.gnba.label(gnba_state_id)
    }

    /// closure + state to vector of LTL
    fn state_to_formulas(&self, state: &State) -> Vec<LTL> {
        state
            .formulas
            .iter()
            .enumerate()
            .filter_map(|(idx, &is_true)| {
                if is_true {
                    Some(self.closure[idx].clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn remove_dead_states(&mut self) {
        if self.states.is_empty() {
            self.transitions.clear();
            return;
        }

        let forward_reachable = self.compute_forward_reachable();
        let backward_reachable = self.compute_backward_reachable();
        let useful_states: FxHashSet<usize> = forward_reachable
            .intersection(&backward_reachable)
            .copied()
            .collect();

        if useful_states.len() == self.states.len() {
            self.transitions = self.generate_transitions();
            return;
        }

        let mut id_map = FxHashMap::default();
        let mut states = Vec::with_capacity(useful_states.len());
        let mut new_to_gnba = Vec::with_capacity(useful_states.len());
        let mut gnba_to_new = FxHashMap::default();

        for (old_id, state) in self.states.iter().enumerate() {
            if useful_states.contains(&old_id) {
                let new_id = states.len();
                id_map.insert(old_id, new_id);
                states.push(State {
                    id: new_id,
                    formulas: state.formulas.clone(),
                });

                let (orig_state_id, acc_id, _) = self.new_to_gnba[old_id];
                new_to_gnba.push((orig_state_id, acc_id, new_id));
                gnba_to_new.insert((orig_state_id, acc_id), new_id);
            }
        }

        self.initial_states = self
            .initial_states
            .iter()
            .filter_map(|state_id| id_map.get(state_id).copied())
            .collect();

        self.acceptance_condition.states = self
            .acceptance_condition
            .states
            .iter()
            .filter_map(|state_id| id_map.get(state_id).copied())
            .collect();

        self.states = states;
        self.new_to_gnba = new_to_gnba;
        self.gnba_to_new = gnba_to_new;
        self.transitions = self.generate_transitions();
    }

    fn compute_forward_reachable(&self) -> FxHashSet<usize> {
        let mut reachable = FxHashSet::default();
        let mut queue = VecDeque::new();

        for &initial_state in &self.initial_states {
            if reachable.insert(initial_state) {
                queue.push_back(initial_state);
            }
        }

        while let Some(state_id) = queue.pop_front() {
            for successor in self.raw_successors(state_id) {
                if reachable.insert(successor) {
                    queue.push_back(successor);
                }
            }
        }

        reachable
    }

    fn compute_backward_reachable(&self) -> FxHashSet<usize> {
        let mut predecessors: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
        for state in &self.states {
            for successor in self.raw_successors(state.id) {
                predecessors.entry(successor).or_default().push(state.id);
            }
        }

        let mut reachable = FxHashSet::default();
        let mut queue = VecDeque::new();

        for &state_id in &self.acceptance_condition.states {
            if reachable.insert(state_id) {
                queue.push_back(state_id);
            }
        }

        while let Some(state_id) = queue.pop_front() {
            if let Some(prev_states) = predecessors.get(&state_id) {
                for &prev_state in prev_states {
                    if reachable.insert(prev_state) {
                        queue.push_back(prev_state);
                    }
                }
            }
        }

        reachable
    }

    /// Generates transitions for the NBA based on the states and closure
    fn generate_transitions(&self) -> Vec<Transition> {
        let mut transitions = Vec::new();
        for from_new_id in 0..self.states.len() {
            let (orig_state_id, acc_id, _) = self.new_to_gnba[from_new_id];
            let label = self.gnba.label(orig_state_id);
            let acc_count = self.gnba.acceptance_conditions.len();
            if acc_count == 0 {
                continue;
            }

            let next_acc_id = (acc_id + 1) % acc_count;
            for to in self.gnba.successors(orig_state_id) {
                if let Some(&to_new) = self.gnba_to_new.get(&(to, next_acc_id)) {
                    transitions.push(Transition {
                        from: from_new_id,
                        to: to_new,
                        label: label.clone(),
                    });
                }
            }
        }
        transitions
    }

    fn raw_successors(&self, state_id: usize) -> Vec<usize> {
        let Some(&(orig_state_id, acc_id, _)) = self.new_to_gnba.get(state_id) else {
            return Vec::new();
        };

        let acc_count = self.gnba.acceptance_conditions.len();
        if acc_count == 0 {
            return Vec::new();
        }

        let next_acc_id = (acc_id + 1) % acc_count;
        let mut successors = Vec::new();
        for to in self.gnba.successors(orig_state_id) {
            if let Some(&to_new) = self.gnba_to_new.get(&(to, next_acc_id)) {
                successors.push(to_new);
            }
        }
        successors
    }
}

impl NBA {
    pub fn pretty_print(&self) {
        println!("Closure:");
        for (idx, formula) in self.closure.iter().enumerate() {
            println!("  {}: {}", idx, formula);
        }

        println!("States:");
        for state in &self.states {
            let formulas = self.state_to_formulas(state);
            println!("  State {}: {:?}", state.id, formulas);
        }

        println!("Initial states: {:?}", self.initial_states);
        println!("Transitions:");
        for transition in &self.transitions {
            println!(
                "  {} --{:?}--> {}",
                transition.from, transition.label, transition.to
            );
        }

        println!("Acceptance conditions:");
        println!(
            "  Condition {}: states {:?}",
            self.acceptance_condition.id, self.acceptance_condition.states
        );
    }

    pub fn to_dot(&self) -> String {
        let atomic_names: Vec<String> = self
            .closure
            .iter()
            .filter_map(|formula| match formula {
                LTL::Var(name) => Some(name.clone()),
                LTL::True => Some("true".to_string()),
                LTL::False => Some("false".to_string()),
                LTL::TokenCount(names) => {
                    if names.len() == 1 {
                        Some(format!("#tokens(\"{}\")", names[0]))
                    } else {
                        Some(format!(
                            "#tokens({})",
                            names
                                .iter()
                                .map(|name| format!("\"{}\"", name))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                    }
                }
                LTL::Fireable(names) => {
                    if names.len() == 1 {
                        Some(format!("\"{}\"?", names[0]))
                    } else {
                        Some(format!(
                            "({})?",
                            names
                                .iter()
                                .map(|name| format!("\"{}\"", name))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                    }
                }
                LTL::LessEqual(_, _)
                | LTL::GreaterEqual(_, _)
                | LTL::Greater(_, _)
                | LTL::Less(_, _) => Some(format!("{}", formula)),
                _ => None,
            })
            .collect();

        let mut s = String::new();
        s.push_str("digraph NBA {\n");
        s.push_str("  rankdir=LR;\n");
        s.push_str("  node [shape=circle];\n");

        // Initial states from start node to initial states
        s.push_str("  start [shape=point];\n");
        for init in &self.initial_states {
            s.push_str(&format!("  start -> {};\n", init));
        }

        for state in &self.states {
            // Only show the state id as node label to keep DOT output concise
            s.push_str(&format!("  {} [label=\"{}\"];\n", state.id, state.id));
        }

        for t in &self.transitions {
            let label_items: Vec<String> = atomic_names
                .iter()
                .zip(t.label.iter())
                .filter_map(|(name, &b)| if b { Some(name.clone()) } else { None })
                .collect();
            let label_str = if label_items.is_empty() {
                "".to_string()
            } else {
                label_items.join(",")
            };
            let escaped = label_str.replace('"', "\\\"");
            s.push_str(&format!(
                "  {} -> {} [label=\"{}\"];\n",
                t.from, t.to, escaped
            ));
        }

        // Mark accepting states
        for st in &self.acceptance_condition.states {
            s.push_str(&format!("  {} [peripheries=2];\n", st));
        }

        s.push_str("}\n");
        s
    }
}
