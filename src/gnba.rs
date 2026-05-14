use crate::closure::compute_closure;
use crate::consistency::is_consistent;
use crate::ltl_parser::LTL;
use crate::pnf::to_pnf;
use std::collections::VecDeque;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct GNBA {
    pub closure: Vec<LTL>,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
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
        let initial_states = find_initial_states(&states, &pnf, &closure);
        let mut acceptance_conditions = generate_acceptance_conditions(&states, &closure);
        if acceptance_conditions.is_empty() {
            acceptance_conditions.push(AcceptanceCondition {
                id: 0,
                states: states.iter().map(|s| s.id).collect(),
            });
        }
        let mut gnba = GNBA {
            closure,
            states,
            transitions: Vec::new(),
            initial_states,
            acceptance_conditions,
        };
        gnba.remove_dead_states();
        gnba
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

    pub fn all_transitions(&self) -> &[Transition] {
        &self.transitions
    }

    pub fn is_accepting(&self, state_id: usize) -> bool {
        self.acceptance_conditions
            .iter()
            .any(|cond| cond.states.contains(&state_id))
    }

    pub fn label(&self, state_id: usize) -> Vec<bool> {
        compute_label(&self.states[state_id], &self.closure)
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

        for state in &self.states {
            if useful_states.contains(&state.id) {
                let new_id = states.len();
                id_map.insert(state.id, new_id);
                states.push(State {
                    id: new_id,
                    formulas: state.formulas.clone(),
                });
            }
        }

        self.initial_states = self
            .initial_states
            .iter()
            .filter_map(|state_id| id_map.get(state_id).copied())
            .collect();

        for condition in &mut self.acceptance_conditions {
            condition.states = condition
                .states
                .iter()
                .filter_map(|state_id| id_map.get(state_id).copied())
                .collect();
        }

        self.states = states;
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

        for condition in &self.acceptance_conditions {
            for &state_id in &condition.states {
                if reachable.insert(state_id) {
                    queue.push_back(state_id);
                }
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

    fn raw_successors(&self, state_id: usize) -> Vec<usize> {
        let from_state = &self.states[state_id];
        let mut successors = Vec::new();
        for to_state in &self.states {
            if is_valid_transition(from_state, to_state, &self.closure) {
                successors.push(to_state.id);
            }
        }
        successors
    }

    fn generate_transitions(&self) -> Vec<Transition> {
        let mut transitions = Vec::new();
        for from_idx in 0..self.states.len() {
            let label = self.label(from_idx);
            for to_idx in 0..self.states.len() {
                if is_valid_transition(&self.states[from_idx], &self.states[to_idx], &self.closure)
                {
                    transitions.push(Transition {
                        from: from_idx,
                        to: to_idx,
                        label: label.clone(),
                    });
                }
            }
        }
        transitions
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
        for transition in self.all_transitions() {
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

        for t in self.all_transitions() {
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


    pub fn to_hoa(&self) -> String {
        let mut hoa = String::new();

        let ap_formulas: Vec<&LTL> = self.closure.iter().filter(|f| match f {
            LTL::Var(_)
            | LTL::TokenCount(_)
            | LTL::Fireable(_)
            | LTL::LessEqual(_, _)
            | LTL::GreaterEqual(_, _)
            | LTL::Greater(_, _)
            | LTL::Less(_, _) => true,
            _ => false,
        }).collect();

        hoa.push_str("HOA: v1\n");
        hoa.push_str(&format!("States: {}\n", self.states.len()));

        for &start_id in &self.initial_states {
            hoa.push_str(&format!("Start: {}\n", start_id));
        }

        hoa.push_str(&format!("AP: {} ", ap_formulas.len()));
        for f in &ap_formulas {
            let name = format!("{}", f).replace('"', "\\\"");
            hoa.push_str(&format!("\"{}\" ", name));
        }
        hoa.push_str("\n");

        let acc_count = self.acceptance_conditions.len();
        if acc_count == 0 {
            hoa.push_str("Acceptance: 1 Inf(0)\n");
        } else {
            let mut acc_expr = String::new();
            for i in 0..acc_count {
                if i > 0 { acc_expr.push_str(" & "); }
                acc_expr.push_str(&format!("Inf({})", i));
            }
            hoa.push_str(&format!("Acceptance: {} {}\n", acc_count, acc_expr));
        }

        hoa.push_str("properties: trans-labels explicit-labels state-acc\n");
        hoa.push_str("--BODY--\n");

        for state in &self.states {
            let mut acc_sets = Vec::new();
            if self.acceptance_conditions.is_empty() {
                acc_sets.push("0".to_string());
            } else {
                for (i, cond) in self.acceptance_conditions.iter().enumerate() {
                    if cond.states.contains(&state.id) {
                        acc_sets.push(i.to_string());
                    }
                }
            }

            let acc_str = if acc_sets.is_empty() {
                String::new()
            } else {
                format!(" {{{}}}", acc_sets.join(" "))
            };

            hoa.push_str(&format!("State: {}{}\n", state.id, acc_str));

            for trans in self.transitions.iter().filter(|t| t.from == state.id) {
                let mut label_parts = Vec::new();
                for (i, &val) in trans.label.iter().enumerate() {
                    if val {
                        label_parts.push(format!("{}", i));
                    } else {
                        label_parts.push(format!("!{}", i));
                    }
                }
                
                let label_str = if label_parts.is_empty() {
                    "[t]".to_string()
                } else {
                    format!("[{}]", label_parts.join(" & "))
                };

                hoa.push_str(&format!("  {} {}\n", label_str, trans.to));
            }
        }

        hoa.push_str("--END--\n");
        hoa
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
fn find_initial_states(states: &[State], formula: &LTL, closure: &[LTL]) -> Vec<usize> {
    let mut initial_states = Vec::new();
    for state in states {
        if let Some(true) = eval_in_state(state, closure, formula) {
            initial_states.push(state.id);
        }
    }
    initial_states
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
        assert!(gnba.states.len() <= 6);
        assert!(!gnba.states.is_empty());
        assert!(
            gnba.initial_states
                .iter()
                .all(|state_id| *state_id < gnba.states.len())
        );
        assert!(gnba.states.iter().all(|state| {
            gnba.successors(state.id)
                .iter()
                .all(|next| *next < gnba.states.len())
        }));
        assert_eq!(gnba.acceptance_conditions.len(), 1);
    }


    #[test]
    fn test_gnba_spot_equivalent() {
        use std::fs;
        use std::process::Command;
        use std::path::Path;

        let spot_bin = std::env::var("SPOT_PATH")
            .unwrap_or_else(|_| "./spot-2.15.1/bin".to_string());
        
        if !Path::new(&spot_bin).exists() {
            eprintln!("Spot not found at {}, skipping test", spot_bin);
            return;
        }

        let test_formulas = vec![
            LTL::Eventually(Box::new(LTL::Var("a".to_string()))),
            LTL::Globally(Box::new(LTL::Var("a".to_string()))),
            LTL::Until(
                Box::new(LTL::Var("a".to_string())),
                Box::new(LTL::Var("b".to_string())),
            ),
            LTL::And(
                Box::new(LTL::Eventually(Box::new(LTL::Var("a".to_string())))),
                Box::new(LTL::Globally(Box::new(LTL::Var("b".to_string())))),
            ),
            LTL::Or(
                Box::new(LTL::Var("a".to_string())),
                Box::new(LTL::Eventually(Box::new(LTL::Var("b".to_string())))),
            ),
        ];

        for (i, formula) in test_formulas.iter().enumerate() {
            let gnba = GNBA::new(formula);
            let hoa_content = gnba.to_hoa();
            
            let our_hoa = format!("/tmp/test_gnba_{}.hoa", i);
            let ref_hoa = format!("/tmp/test_gnba_{}_ref.hoa", i);
            
            fs::write(&our_hoa, &hoa_content)
                .expect("Failed to write GNBA HOA file");
            
            let formula_str = formula_to_spot_ltl(formula);
            
            let ltl2tgba = format!("{}/ltl2tgba", spot_bin);
            let output = Command::new(&ltl2tgba)
                .arg("-H")
                .arg(&formula_str)
                .output()
                .expect("Failed to run ltl2tgba");
            
            if !output.status.success() {
                eprintln!("ltl2tgba failed for formula: {}", formula_str);
                eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                fs::remove_file(&our_hoa).ok();
                continue;
            }
            
            fs::write(&ref_hoa, &output.stdout)
                .expect("Failed to write reference HOA file");
            
            let autfilt = format!("{}/autfilt", spot_bin);
            let equiv_check = Command::new(&autfilt)
                .arg(format!("--equivalent-to={}", our_hoa))
                .arg(&ref_hoa)
                .output()
                .expect("Failed to run autfilt");
            
            fs::remove_file(&our_hoa).ok();
            fs::remove_file(&ref_hoa).ok();
            
            assert!(
                equiv_check.status.success(),
                "Formula {} not equivalent: GNBA vs Spot reference\nFormula: {}\nStdout: {}\nStderr: {}",
                i,
                formula_str,
                String::from_utf8_lossy(&equiv_check.stdout),
                String::from_utf8_lossy(&equiv_check.stderr)
            );
        }
    }

    fn formula_to_spot_ltl(formula: &LTL) -> String {
        match formula {
            LTL::True => "1".to_string(),
            LTL::False => "0".to_string(),
            LTL::Var(name) => name.clone(),
            LTL::Not(inner) => format!("!({})", formula_to_spot_ltl(inner)),
            LTL::And(left, right) => format!("({} & {})", formula_to_spot_ltl(left), formula_to_spot_ltl(right)),
            LTL::Or(left, right) => format!("({} | {})", formula_to_spot_ltl(left), formula_to_spot_ltl(right)),
            LTL::Implies(left, right) => format!("({} -> {})", formula_to_spot_ltl(left), formula_to_spot_ltl(right)),
            LTL::Next(inner) => format!("X ({})", formula_to_spot_ltl(inner)),
            LTL::Eventually(inner) => format!("F ({})", formula_to_spot_ltl(inner)),
            LTL::Globally(inner) => format!("G ({})", formula_to_spot_ltl(inner)),
            LTL::Until(left, right) => format!("({} U {})", formula_to_spot_ltl(left), formula_to_spot_ltl(right)),
            LTL::Release(left, right) => format!("({} R {})", formula_to_spot_ltl(left), formula_to_spot_ltl(right)),
            LTL::WeakUntil(left, right) => format!("({} W {})", formula_to_spot_ltl(left), formula_to_spot_ltl(right)),
            LTL::TokenCount(_) | LTL::Fireable(_) | LTL::LessEqual(_, _) | LTL::GreaterEqual(_, _) | LTL::Greater(_, _) | LTL::Less(_, _) => {
                "1".to_string()
            }
            _ => "1".to_string(),
        }
    }
}


