use crate::gnba::GNBA;
use crate::ltl_parser::LTL;
use std::cell::RefCell;

pub struct NBA {
    pub closure: Vec<LTL>,
    pub states: Vec<State>,
    pub initial_states: Vec<usize>,
    pub acceptance_condition: AcceptanceCondition,
    new_to_gnba: Vec<(usize, usize, usize)>,
    gnba_to_new: std::collections::HashMap<(usize, usize), usize>,
    gnba: GNBA,
    successors_cache: RefCell<Vec<Option<Vec<usize>>>>,
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
        let mut gnba_to_new = std::collections::HashMap::new();
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

        let state_count = states.len();

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

        NBA {
            closure,
            states,
            initial_states,
            acceptance_condition,
            new_to_gnba,
            gnba_to_new,
            gnba,
            successors_cache: RefCell::new(vec![None; state_count]),
        }
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

        self.cached_successors(state_id)
    }

    pub fn successors(&self, state_id: usize) -> Vec<usize> {
        self.cached_successors(state_id)
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

    /// Generates transitions for the NBA based on the states and closure
    fn generate_transitions(&self) -> Vec<Transition> {
        let mut transitions = Vec::new();
        // Build transitions from cached NBA successors and GNBA labels
        for from_new_id in 0..self.states.len() {
            let (orig_state_id, _acc_id, _new_id) = self.new_to_gnba[from_new_id];
            let label = self.gnba.label(orig_state_id);
            for to_new in self.cached_successors(from_new_id) {
                transitions.push(Transition {
                    from: from_new_id,
                    to: to_new,
                    label: label.clone(),
                });
            }
        }
        transitions
    }

    fn cached_successors(&self, state_id: usize) -> Vec<usize> {
        if let Some(successors) = self
            .successors_cache
            .borrow()
            .get(state_id)
            .and_then(|entry| entry.clone())
        {
            return successors;
        }

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

        self.successors_cache.borrow_mut()[state_id] = Some(successors.clone());
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
        for transition in &self.generate_transitions() {
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

        for t in &self.generate_transitions() {
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
