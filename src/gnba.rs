use crate::closure::compute_closure;
use crate::consistency::is_consistent;
use crate::ltl_parser::LTL;
use crate::pnf::to_pnf;

pub struct GNBA {
    pub closure: Vec<LTL>,
    pub states: Vec<State>,
    pub initial_states: Vec<usize>,
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
            acceptance_conditions,
        }
    }

    pub fn successors(&self, state_id: usize) -> Vec<usize> {
        let mut result = Vec::new();
        let from_state = &self.states[state_id];

        for to_state in &self.states {
            if is_valid_transition(from_state, to_state, &self.closure) {
                result.push(to_state.id);
            }
        }

        result
    }

    pub fn all_transitions(&self) -> Vec<Transition> {
        let mut transitions = Vec::new();
        for from_state in &self.states {
            for to_state in &self.states {
                if is_valid_transition(from_state, to_state, &self.closure) {
                    let label = compute_label(from_state, &self.closure);
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

    pub fn is_accepting(&self, state_id: usize) -> bool {
        self.acceptance_conditions
            .iter()
            .any(|cond| cond.states.contains(&state_id))
    }
}

impl GNBA {
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
        for transition in &generate_transitions(&self.states, &self.closure) {
            println!(
                "  {} --{:?}--> {}",
                transition.from, transition.label, transition.to
            );
        }

        println!("Acceptance conditions:");
        for condition in &self.acceptance_conditions {
            println!(
                "  Condition {}: states {:?}",
                condition.id, condition.states
            );
        }
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
                                .map(|name| format!("\"{}\"" , name))
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

        for t in &generate_transitions(&self.states, &self.closure) {
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

        for condition in &self.acceptance_conditions {
            for st in &condition.states {
                s.push_str(&format!("  {} [peripheries=2];\n", st));
            }
        }

        s.push_str("}\n");
        s
    }
}

/// closure + state to vector of LTL
fn state_to_formulas(state: &State, closure: &[LTL]) -> Vec<LTL> {
    state
        .formulas
        .iter()
        .enumerate()
        .filter_map(|(idx, &is_true)| {
            if is_true {
                Some(closure[idx].clone())
            } else {
                None
            }
        })
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

fn eval_in_state(state: &State, closure: &[LTL], formula: &LTL) -> Option<bool> {
    if let Some(idx) = closure.iter().position(|f| f == formula) {
        Some(state.formulas[idx])
    } else if let LTL::Not(inner) = formula {
        closure
            .iter()
            .position(|f| f == inner.as_ref())
            .map(|idx| !state.formulas[idx])
    } else {
        None
    }
}

/// Check if a transition from one state to another satisfies the transition rules
fn is_valid_transition(from_state: &State, to_state: &State, closure: &[LTL]) -> bool {
    for (idx, formula) in closure.iter().enumerate() {
        match formula {
            LTL::Next(inner) => {
                if let Some(inner_next) = eval_in_state(to_state, closure, inner.as_ref()) {
                    let x_curr = from_state.formulas[idx];

                    if x_curr != inner_next {
                        return false;
                    }
                }
            }
            LTL::Until(left, right) => {
                let left_curr = eval_in_state(from_state, closure, left.as_ref()).unwrap_or(false);
                let right_curr =
                    eval_in_state(from_state, closure, right.as_ref()).unwrap_or(false);

                let u_curr = from_state.formulas[idx];
                let u_next = eval_in_state(to_state, closure, formula).unwrap_or(false);

                if u_curr != (right_curr || (left_curr && u_next)) {
                    return false;
                }
            }
            LTL::Release(left, right) => {
                let left_curr = eval_in_state(from_state, closure, left.as_ref()).unwrap_or(false);
                let right_curr =
                    eval_in_state(from_state, closure, right.as_ref()).unwrap_or(false);

                let r_curr = from_state.formulas[idx];
                let r_next = eval_in_state(to_state, closure, formula).unwrap_or(false);

                if r_curr != (right_curr && (left_curr || r_next)) {
                    return false;
                }
            }
            _ => {}
        }
    }

    true
}

/// Compute the label of a state (which atomic propositions are true)
fn compute_label(state: &State, closure: &[LTL]) -> Vec<bool> {
    closure
        .iter()
        .enumerate()
        .filter_map(|(idx, formula)| match formula {
            LTL::Var(_)
            | LTL::True
            | LTL::False
            | LTL::Fireable(_)
            | LTL::LessEqual(_, _)
            | LTL::GreaterEqual(_, _)
            | LTL::Greater(_, _)
            | LTL::Less(_, _) => Some(state.formulas[idx]),
            _ => None,
        })
        .collect()
}

/// Generate acceptance conditions (one for each until-subformula)
fn generate_acceptance_conditions(states: &[State], closure: &[LTL]) -> Vec<AcceptanceCondition> {
    let mut acceptance_conditions = Vec::new();
    let mut condition_id = 0;

    for (idx, formula) in closure.iter().enumerate() {
        if let LTL::Until(_left, right) = formula {
            let accepting_states: Vec<usize> = states
                .iter()
                .filter_map(|state| {
                    let until_false = !state.formulas[idx];

                    // Check if right side is true
                    let right_true = match right.as_ref() {
                        LTL::Not(inner) => {
                            // For negated formulas not in closure, check inner and negate
                            closure
                                .iter()
                                .position(|f| f == inner.as_ref())
                                .is_some_and(|i| !state.formulas[i])
                        }
                        other => {
                            // For formulas in closure, check directly
                            closure
                                .iter()
                                .position(|f| f == other)
                                .is_some_and(|i| state.formulas[i])
                        }
                    };

                    if until_false || right_true {
                        Some(state.id)
                    } else {
                        None
                    }
                })
                .collect();

            acceptance_conditions.push(AcceptanceCondition {
                id: condition_id,
                states: accepting_states,
            });
            condition_id += 1;
        }
    }

    acceptance_conditions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gnba_construction() {
        let formula = LTL::Until(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::And(
                Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
                Box::new(LTL::Var("b".to_string())),
            )),
        );

        let gnba = GNBA::new(&formula);
        assert_eq!(gnba.closure.len(), 4);
        assert_eq!(gnba.states.len(), 6);
        assert_eq!(gnba.initial_states.len(), 3);
        assert_eq!(gnba.all_transitions().len(), 24);
        assert_eq!(gnba.acceptance_conditions.len(), 1);
    }
}
