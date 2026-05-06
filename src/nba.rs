use crate::gnba::GNBA;
use crate::ltl_parser::LTL;

pub struct NBA {
    pub closure: Vec<LTL>,
    pub states: Vec<State>,
    pub initial_states: Vec<usize>,
    pub transitions: Vec<Transition>,
    pub acceptance_condition: AcceptanceCondition,
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
        let mut state_mapping = Vec::new(); // Maps (original_state_id, acceptance_condition_id) to new_state_id
        let mut new_state_id = 0;

        for state in &gnba.states {
            for acc_id in 0..gnba.acceptance_conditions.len() {
                states.push(State {
                    id: new_state_id,
                    formulas: state.formulas.clone(),
                });
                state_mapping.push((state.id, acc_id, new_state_id));
                new_state_id += 1;
            }
        }

        let initial_states: Vec<usize> = gnba.initial_states.iter().map(|orig_id| {
            state_mapping.iter().find(|(orig_id_map, acc, _)| *orig_id_map == *orig_id && *acc == 0).unwrap().2
        }).collect();

        // Accepting state: first of the gnba
        let acceptance_condition = AcceptanceCondition {
            id: 0,
            states: state_mapping.iter().filter_map(|(orig_id, acc, new_id)| {
                if gnba.acceptance_conditions[0].states.contains(orig_id) && *acc == 0 {
                    Some(*new_id)
                } else {
                    None
                }
            }).collect(),
        };
        

        // Create transitions between the new states fromt gnba 1 to gnba 2 etc
        let mut transitions = Vec::new();
        for transition in &gnba.transitions {
            for acc_id in 0..gnba.acceptance_conditions.len() {
                let from_new_id = state_mapping.iter().find(|(orig_id, acc, _)| *orig_id == transition.from && *acc == acc_id).unwrap().2;
                let to_new_id = state_mapping.iter().find(|(orig_id, acc, _)| *orig_id == transition.to && *acc == (acc_id+1) % gnba.acceptance_conditions.len()).unwrap().2;
                transitions.push(Transition {
                    from: from_new_id,
                    to: to_new_id,
                    label: transition.label.clone(),
                });
            }
        }


        NBA {
            closure: gnba.closure,
            states,
            initial_states,
            transitions,
            acceptance_condition,
        }

    }


    pub fn initial_states(&self) -> Vec<usize> {
        self.initial_states.clone()
    }

    pub fn is_accepting(&self, state_id: usize) -> bool {
        self.acceptance_condition.states.contains(&state_id)
    }


    pub fn next(&self, state_id: usize, label: &[bool]) -> Vec<usize> {
        self.transitions.iter()
            .filter(|t| t.from == state_id && t.label == label)
            .map(|t| t.to)
            .collect()
    }


    pub fn pretty_print(&self) {
        println!("Closure:");
        for (idx, formula) in self.closure.iter().enumerate() {
            println!("  {}: {}", idx, formula);
        }

        println!("States:");
        for state in &self.states {
            let formulas = state_to_formulas(state, &self.closure);
            println!("  State {}: {:?}", state.id, formulas);
        }

        println!("Initial states: {:?}", self.initial_states);
        println!("Transitions:");
        for transition in &self.transitions {
            println!("  {} --{:?}--> {}", transition.from, transition.label, transition.to);
        }

        println!("Acceptance conditions:");
        println!("  Condition {}: states {:?}", self.acceptance_condition.id, self.acceptance_condition.states);
    }


}

impl NBA {
    pub fn to_dot(&self) -> String {
        let atomic_names: Vec<String> = self.closure.iter().filter_map(|formula| {
            match formula {
                LTL::Var(name) => Some(name.clone()),
                LTL::True => Some("true".to_string()),
                LTL::False => Some("false".to_string()),
                LTL::Fireable(name) => Some(format!("\"{}\"?", name)),
                LTL::LessEqual(_, _) | LTL::GreaterEqual(_, _) | LTL::Greater(_, _) | LTL::Less(_, _) => {
                    Some(format!("{}", formula))
                }
                _ => None,
            }
        }).collect();

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
            let label_items: Vec<String> = atomic_names.iter().zip(t.label.iter())
                .filter_map(|(name, &b)| if b { Some(name.clone()) } else { None })
                .collect();
            let label_str = if label_items.is_empty() { "".to_string() } else { label_items.join(",") };
            let escaped = label_str.replace('"', "\\\"");
            s.push_str(&format!("  {} -> {} [label=\"{}\"];\n", t.from, t.to, escaped));
        }

        // Mark accepting states
        for st in &self.acceptance_condition.states {
            s.push_str(&format!("  {} [peripheries=2];\n", st));
        }


        s.push_str("}\n");
        s
    }
}

/// closure + state to vector of LTL
fn state_to_formulas(state: &State, closure: &[LTL]) -> Vec<LTL> {
    state.formulas.iter().enumerate()
        .filter_map(|(idx, &is_true)| if is_true { Some(closure[idx].clone()) } else { None })
        .collect()
}