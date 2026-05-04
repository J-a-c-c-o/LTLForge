use crate::ltl_parser::LTL;
use crate::pnf::to_pnf;
use crate::closure::compute_closure;
use crate::consistency::is_consistent;


pub struct GNBA {
    pub closure: Vec<LTL>,
    pub states: Vec<State>,
    pub initial_states: Vec<usize>,
    pub transitions: Vec<Transition>,
    pub acceptance_conditions: Vec<AcceptanceCondition>,
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

impl GNBA {
    pub fn new(ltl: &LTL) -> Self {
        let pnf = to_pnf(ltl);
        let closure = compute_closure(&pnf);
        let states = generate_states(&closure);
        let initial_states = find_initial_states(&states);
        let transitions = generate_transitions(&states, &closure);
        let mut acceptance_conditions = generate_acceptance_conditions(&states, &closure);
        if acceptance_conditions.is_empty() {
            acceptance_conditions.push(AcceptanceCondition {
                id: 0,
                states: states.iter().map(|s| s.id).collect(),
            });
        }
        GNBA {
            closure,
            states,
            initial_states,
            transitions,
            acceptance_conditions,
        }
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
        for condition in &self.acceptance_conditions {
            println!("  Condition {}: states {:?}", condition.id, condition.states);
        }
    }
}

/// closure + state to vector of LTL
fn state_to_formulas(state: &State, closure: &[LTL]) -> Vec<LTL> {
    state.formulas.iter().enumerate()
        .filter_map(|(idx, &is_true)| if is_true { Some(closure[idx].clone()) } else { None })
        .collect()
}

/// Generate all locally consistent states from the closure
fn generate_states(closure: &[LTL]) -> Vec<State> {
    let consistent_truth_assignments = is_consistent(closure);
    let mut states = Vec::new();
    
    for (id, formulas) in consistent_truth_assignments.iter().enumerate() {
        states.push(State {
            id,
            formulas: formulas.clone(),
        });
    }
    
    states
}

/// Find the initial states (those containing the formula)
fn find_initial_states(states: &[State]) -> Vec<usize> {
    let mut initial_states = Vec::new();
    for state in states {
        let len = state.formulas.len();
        if state.formulas[len - 1] {
            initial_states.push(state.id);
        }
    }
    initial_states
}

/// Generate transitions following X, U, R expansion rules
fn generate_transitions(states: &[State], closure: &[LTL]) -> Vec<Transition> {
    let mut transitions = Vec::new();
    
    for from_state in states {
        // Find all valid successor states
        for to_state in states {
            if is_valid_transition(from_state, to_state, closure) {
                // Calculate the label (atomic propositions true in from_state)
                let label = compute_label(from_state, closure);
                transitions.push(Transition {
                    from: from_state.id,
                    to: to_state.id,
                    label,
                });
            }
        }
    }
    
    transitions
}

/// Check if a transition from one state to another satisfies the transition rules
fn is_valid_transition(from_state: &State, to_state: &State, closure: &[LTL]) -> bool {
    for (idx, formula) in closure.iter().enumerate() {
        match formula {
            LTL::Next(inner) => {
                if let Some(inner_idx) = closure.iter().position(|f| f == inner.as_ref()) {
                    let x_curr = from_state.formulas[idx];
                    let inner_next = to_state.formulas[inner_idx];
                    
                    if x_curr != inner_next {
                        return false;
                    }
                }
            }
            LTL::Until(left, right) => {
                let left_curr = closure.iter().position(|f| f == left.as_ref())
                    .map_or(false, |i| from_state.formulas[i]);
                let right_curr = closure.iter().position(|f| f == right.as_ref())
                    .map_or(false, |i| from_state.formulas[i]);
                
                let u_curr = from_state.formulas[idx];
                let u_next = to_state.formulas[idx];

                if u_curr != (right_curr || (left_curr && u_next)) {
                    return false;
                }
            }
            LTL::Release(left, right) => {
                let left_curr = closure.iter().position(|f| f == left.as_ref())
                    .map_or(false, |i| from_state.formulas[i]);
                let right_curr = closure.iter().position(|f| f == right.as_ref())
                    .map_or(false, |i| from_state.formulas[i]);
                
                let r_curr = from_state.formulas[idx];
                let r_next = to_state.formulas[idx];

                if r_curr != (right_curr && (left_curr || r_next)) {
                    return false;
                }
            }
            _ => {

            }
        }
    }
    
    true
}


/// Compute the label of a state (which atomic propositions are true)
fn compute_label(state: &State, closure: &[LTL]) -> Vec<bool> {
    closure.iter().enumerate()
        .filter_map(|(idx, formula)| {
            match formula {
                LTL::Var(_) => Some(state.formulas[idx]),
                LTL::True => Some(true),
                LTL::False => Some(false),
                LTL::LessEqual(_, _) => Some(state.formulas[idx]),
                LTL::GreaterEqual(_, _) => Some(state.formulas[idx]),
                LTL::Greater(_, _) => Some(state.formulas[idx]),
                LTL::Less(_, _) => Some(state.formulas[idx]),
                _ => None,
            }
        })
        .collect()
}

/// Generate acceptance conditions (one for each until-subformula)
fn generate_acceptance_conditions(states: &[State], closure: &[LTL]) -> Vec<AcceptanceCondition> {
    let mut acceptance_conditions = Vec::new();
    let mut condition_id = 0;
    
    for (idx, formula) in closure.iter().enumerate() {
        if let LTL::Until(_left, right) = formula {
            // F_φUψ = {S ∈ CS(φ) | φ U ψ ∉ S ∨ ψ ∈ S}
            let right_idx = closure.iter().position(|f| f == right.as_ref());
            let mut accepting_states = Vec::new();
            
            for state in states {
                let until_false = !state.formulas[idx];
                let right_true = right_idx.map_or(false, |i| state.formulas[i]);
                
                if until_false || right_true {
                    accepting_states.push(state.id);
                }
            }
            
            acceptance_conditions.push(AcceptanceCondition {
                id: condition_id,
                states: accepting_states,
            });
            condition_id += 1;
        }
    }
    
    acceptance_conditions
}

impl GNBA {
    pub fn to_dot(&self) -> String {
        let atomic_names: Vec<String> = self.closure.iter().filter_map(|formula| {
            match formula {
                LTL::Var(name) => Some(name.clone()),
                LTL::True => Some("true".to_string()),
                LTL::False => Some("false".to_string()),
                LTL::LessEqual(_, _) | LTL::GreaterEqual(_, _) | LTL::Greater(_, _) | LTL::Less(_, _) => {
                    Some(format!("{}", formula))
                }
                _ => None,
            }
        }).collect();

        let mut s = String::new();
        s.push_str("digraph GNBA {\n");
        s.push_str("  rankdir=LR;\n");
        s.push_str("  start [shape=point];\n");

        for state in &self.states {
            // Only show the state id as node label — formulas are often unreadable
            s.push_str(&format!("  {} [label=\"{}\"];\n", state.id, state.id));
        }

        for init in &self.initial_states {
            s.push_str(&format!("  start -> {};\n", init));
        }

        for t in &self.transitions {
            let label_items: Vec<String> = atomic_names.iter().zip(t.label.iter())
                .filter_map(|(name, &b)| if b { Some(name.clone()) } else { None })
                .collect();
            let label_str = if label_items.is_empty() { "".to_string() } else { label_items.join(",") };
            let escaped = label_str.replace('"', "\\\"");
            s.push_str(&format!("  {} -> {} [label=\"{}\"];\n", t.from, t.to, escaped));
        }

        for condition in &self.acceptance_conditions {
            for st in &condition.states {
                s.push_str(&format!("  {} [peripheries=2];\n", st));
            }
        }

        s.push_str("}\n");
        s
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_GNBA_construction() {
        let formula = LTL::Until(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::And(
                Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
                Box::new(LTL::Var("b".to_string())),
            )),
        );

        let gnba = GNBA::new(&formula);
        println!("Closure: {:?}", gnba.closure);
        println!("States:");
        for state in &gnba.states {
            println!("  State {}: {:?}", state.id, state.formulas);
        }

        println!("Initial states: {:?}", gnba.initial_states);
        println!("Transitions:");
        for transition in &gnba.transitions {
            println!("  {} --{:?}--> {}", transition.from, transition.label, transition.to);
        }

        println!("Acceptance conditions:");
        for condition in &gnba.acceptance_conditions {
            println!("  Condition {}: states {:?}", condition.id, condition.states);
        }

        panic!("Test complete - manual verification needed");
    }
}