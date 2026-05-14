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

            let next_acc_id = if self
                .gnba
                .acceptance_conditions[acc_id]
                .states
                .contains(&orig_state_id)
            {
                (acc_id + 1) % acc_count
            } else {
                acc_id
            };

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

        let next_acc_id = if self
            .gnba
            .acceptance_conditions[acc_id]
            .states
            .contains(&orig_state_id)
        {
            (acc_id + 1) % acc_count
        } else {
            acc_id
        };

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
    pub fn print_stats(&self) {
        println!("NBA:");
        println!("States: {}", self.states.len());
        println!("Initial States: {}", self.initial_states.len());
        println!("Accepting States: {}", self.acceptance_condition.states.len());
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
            // Format formula and escape quotes for HOA compatibility
            let name = format!("{}", f).replace('"', "\\\"");
            hoa.push_str(&format!("\"{}\" ", name));
        }
        hoa.push_str("\n");


        hoa.push_str("Acceptance: 1 Inf(0)\n");
        
        hoa.push_str("properties: trans-labels explicit-labels state-acc\n");
        hoa.push_str("--BODY--\n");

        for state in &self.states {
            let acc_str = if self.acceptance_condition.states.contains(&state.id) {
                " {0}"
            } else {
                ""
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nba_spot_equivalent() {
        use std::fs;
        use std::process::Command;
        use std::path::Path;

        // Spot installation path
        let spot_bin = std::env::var("SPOT_PATH")
            .unwrap_or_else(|_| "./spot-2.15.1/bin".to_string());
        
        if !Path::new(&spot_bin).exists() {
            eprintln!("Spot not found at {}, skipping test", spot_bin);
            return;
        }

        let mut test_formulas = Vec::new();
        for _ in 0..100 {
            let formula = random_ltl(2);
            test_formulas.push(formula);
        }

        for (i, formula) in test_formulas.iter().enumerate() {
            let nba = NBA::new(formula);
            let hoa_content = nba.to_hoa();
            
            let our_hoa = format!("/tmp/test_NBA_{}.hoa", i);
            let ref_hoa = format!("/tmp/test_NBA_{}_ref.hoa", i);
            
            fs::write(&our_hoa, &hoa_content)
                .expect("Failed to write NBA HOA file");
            
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
                "Formula {} not equivalent: NBA vs Spot reference\nFormula: {}\nStdout: {}\nStderr: {}",
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



    fn random_ltl(depth: usize) -> LTL {
        if depth == 0 {
            let var_name = format!("p{}", rand::random_range(0..10));
            return LTL::Var(var_name);
        }

        let choice = rand::random_range(0..10);
        match choice {
            0 => LTL::Not(Box::new(random_ltl(depth - 1))),
            1 => LTL::And(Box::new(random_ltl(depth - 1)), Box::new(random_ltl(depth - 1))),
            2 => LTL::Or(Box::new(random_ltl(depth - 1)), Box::new(random_ltl(depth - 1))),
            3 => LTL::Or(Box::new(LTL::Not(Box::new(random_ltl(depth - 1)))), Box::new(random_ltl(depth - 1))),
            4 => LTL::Next(Box::new(random_ltl(depth - 1))),
            5 => LTL::Eventually(Box::new(random_ltl(depth - 1))),
            6 => LTL::Globally(Box::new(random_ltl(depth - 1))),
            _ => LTL::Until(Box::new(random_ltl(depth - 1)), Box::new(random_ltl(depth - 1))),
        }
    }
}