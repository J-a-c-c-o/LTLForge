use crate::gnba::GNBA;
use crate::ltl_parser::LTL;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

pub struct NBA {
    pub closure: Vec<LTL>,
    pub states: Vec<State>,
    pub transitions: Vec<Transitions>,
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

#[derive(Clone)]
pub struct Transitions {
    pub transitions: Vec<usize>,
    pub label: Vec<Vec<bool>>,
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
        nba.transitions = nba.generate_transitions();
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

        self.successors(state_id).to_vec()
    }

    pub fn successors(&self, state_id: usize) -> &[usize] {
        &self.transitions[state_id].transitions
    }

    fn gnba_state_label(&self, gnba_state_id: usize) -> Vec<bool> {
        self.gnba.label(gnba_state_id)
    }

    fn remove_dead_states(&mut self) {
        let mut reachable_from_initial = FxHashSet::default();
        let mut worklist = VecDeque::new();

        for &initial in &self.initial_states {
            if reachable_from_initial.insert(initial) {
                worklist.push_back(initial);
            }
        }

        while let Some(state_id) = worklist.pop_front() {
            for &next in self.successors(state_id) {
                if reachable_from_initial.insert(next) {
                    worklist.push_back(next);
                }
            }
        }

        let sccs = Self::compute_sccs_nba(self);
        let mut accepting_cycle_states = FxHashSet::default();

        for scc in sccs {
            let is_cyclic = if scc.len() > 1 {
                true
            } else {
                let state_id = scc[0];
                self.successors(state_id).contains(&state_id)
            };

            if is_cyclic && scc.iter().any(|state_id| self.is_accepting(*state_id)) {
                accepting_cycle_states.extend(scc);
            }
        }

        let mut can_reach_accepting = FxHashSet::default();
        let mut reverse_edges: Vec<Vec<usize>> = vec![Vec::new(); self.states.len()];

        for (from_id, trans_entry) in self.transitions.iter().enumerate() {
            for &to_id in &trans_entry.transitions {
                reverse_edges[to_id].push(from_id);
            }
        }

        let mut worklist: VecDeque<usize> = accepting_cycle_states.iter().copied().collect();
        while let Some(state_id) = worklist.pop_front() {
            if !can_reach_accepting.insert(state_id) {
                continue;
            }

            for &pred in &reverse_edges[state_id] {
                if !can_reach_accepting.contains(&pred) {
                    worklist.push_back(pred);
                }
            }
        }

        let useful_states: FxHashSet<usize> = reachable_from_initial
            .intersection(&can_reach_accepting)
            .copied()
            .collect();

        let old_transitions = self.transitions.clone();
        let mut old_to_new = vec![None; self.states.len()];
        let mut new_states = Vec::with_capacity(useful_states.len());
        let mut new_to_gnba = Vec::with_capacity(useful_states.len());
        let mut gnba_to_new = FxHashMap::default();

        for old_id in 0..self.states.len() {
            if !useful_states.contains(&old_id) {
                continue;
            }

            let new_id = new_states.len();
            old_to_new[old_id] = Some(new_id);
            new_states.push(State {
                id: new_id,
                formulas: self.states[old_id].formulas.clone(),
            });

            let (orig_state_id, acc_id, _) = self.new_to_gnba[old_id];
            new_to_gnba.push((orig_state_id, acc_id, new_id));
            gnba_to_new.insert((orig_state_id, acc_id), new_id);
        }

        self.initial_states = self
            .initial_states
            .iter()
            .filter_map(|&old_id| old_to_new[old_id])
            .collect();

        self.acceptance_condition.states = self
            .acceptance_condition
            .states
            .iter()
            .filter_map(|&old_id| old_to_new[old_id])
            .collect();

        let mut new_transitions = vec![
            Transitions {
                transitions: Vec::new(),
                label: Vec::new(),
            };
            new_states.len()
        ];

        for old_from in 0..old_transitions.len() {
            let Some(new_from) = old_to_new[old_from] else {
                continue;
            };

            for (old_to, label) in old_transitions[old_from]
                .transitions
                .iter()
                .copied()
                .zip(old_transitions[old_from].label.iter())
            {
                if let Some(new_to) = old_to_new[old_to] {
                    new_transitions[new_from].transitions.push(new_to);
                    new_transitions[new_from].label.push(label.clone());
                }
            }
        }

        self.states = new_states;
        self.transitions = new_transitions;
        self.new_to_gnba = new_to_gnba;
        self.gnba_to_new = gnba_to_new;
    }

    fn compute_sccs_nba(nba: &NBA) -> Vec<Vec<usize>> {
        let n = nba.states.len();
        let mut index = vec![None; n];
        let mut lowlink = vec![0usize; n];
        let mut stack: Vec<usize> = Vec::new();
        let mut onstack = vec![false; n];
        let mut current_index: usize = 0;
        let mut sccs: Vec<Vec<usize>> = Vec::new();

        fn strongconnect(
            v: usize,
            nba: &NBA,
            index: &mut [Option<usize>],
            lowlink: &mut [usize],
            stack: &mut Vec<usize>,
            onstack: &mut [bool],
            current_index: &mut usize,
            sccs: &mut Vec<Vec<usize>>,
        ) {
            index[v] = Some(*current_index);
            lowlink[v] = *current_index;
            *current_index += 1;
            stack.push(v);
            onstack[v] = true;

            for &w in nba.successors(v) {
                if index[w].is_none() {
                    strongconnect(w, nba, index, lowlink, stack, onstack, current_index, sccs);
                    lowlink[v] = std::cmp::min(lowlink[v], lowlink[w]);
                } else if onstack[w] {
                    lowlink[v] = std::cmp::min(lowlink[v], index[w].unwrap());
                }
            }

            if index[v].unwrap() == lowlink[v] {
                let mut scc = Vec::new();
                loop {
                    let w = stack.pop().unwrap();
                    onstack[w] = false;
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }
                sccs.push(scc);
            }
        }

        for v in 0..n {
            if index[v].is_none() {
                strongconnect(
                    v,
                    nba,
                    &mut index,
                    &mut lowlink,
                    &mut stack,
                    &mut onstack,
                    &mut current_index,
                    &mut sccs,
                );
            }
        }

        sccs
    }

    /// Generates transitions for the NBA based on the states and closure
    fn generate_transitions(&self) -> Vec<Transitions> {
        let mut transitions = vec![
            Transitions {
                transitions: Vec::new(),
                label: Vec::new(),
            };
            self.states.len()
        ];

        for from_new_id in 0..self.states.len() {
            let (orig_state_id, acc_id, _) = self.new_to_gnba[from_new_id];
            let label = self.gnba.label(orig_state_id);
            let acc_count = self.gnba.acceptance_conditions.len();
            if acc_count == 0 {
                continue;
            }

            let next_acc_id = if self.gnba.acceptance_conditions[acc_id]
                .states
                .contains(&orig_state_id)
            {
                (acc_id + 1) % acc_count
            } else {
                acc_id
            };

            for &to in self.gnba.successors(orig_state_id) {
                if let Some(&to_new) = self.gnba_to_new.get(&(to, next_acc_id)) {
                    transitions[from_new_id].transitions.push(to_new);
                    transitions[from_new_id].label.push(label.clone());
                }
            }
        }
        transitions
    }
}

impl NBA {
    pub fn print_stats(&self) {
        println!("NBA:");
        println!("States: {}", self.states.len());
        println!("Initial States: {}", self.initial_states.len());
        println!(
            "Accepting States: {}",
            self.acceptance_condition.states.len()
        );
        println!("Transitions: {}", self.transitions.len());
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

        for (from_id, trans_entry) in self.transitions.iter().enumerate() {
            for (to_id, label) in trans_entry.transitions.iter().zip(trans_entry.label.iter()) {
                let label_items: Vec<String> = atomic_names
                    .iter()
                    .zip(label.iter())
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
                    from_id, to_id, escaped
                ));
            }
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

        let ap_formulas: Vec<&LTL> = self
            .closure
            .iter()
            .filter(|f| {
                matches!(
                    f,
                    LTL::Var(_)
                        | LTL::TokenCount(_)
                        | LTL::Fireable(_)
                        | LTL::LessEqual(_, _)
                        | LTL::GreaterEqual(_, _)
                        | LTL::Greater(_, _)
                        | LTL::Less(_, _)
                )
            })
            .collect();

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
        hoa.push('\n');

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

            let trans_entry = &self.transitions[state.id];
            for (to_id, label) in trans_entry.transitions.iter().zip(trans_entry.label.iter()) {
                let mut label_parts = Vec::new();
                for (i, &val) in label.iter().enumerate() {
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

                hoa.push_str(&format!("  {} {}\n", label_str, to_id));
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
    fn test_nba_dead_state_removal_prunes_unreachable_and_non_accepting_states() {
        let dummy_gnba = GNBA::new(&LTL::True);

        let mut nba = NBA {
            closure: vec![],
            states: vec![
                State {
                    id: 0,
                    formulas: vec![],
                },
                State {
                    id: 1,
                    formulas: vec![],
                },
                State {
                    id: 2,
                    formulas: vec![],
                },
                State {
                    id: 3,
                    formulas: vec![],
                },
                State {
                    id: 4,
                    formulas: vec![],
                },
            ],
            transitions: vec![
                Transitions {
                    transitions: vec![1, 3],
                    label: vec![vec![], vec![]],
                },
                Transitions {
                    transitions: vec![2],
                    label: vec![vec![]],
                },
                Transitions {
                    transitions: vec![],
                    label: vec![],
                },
                Transitions {
                    transitions: vec![4],
                    label: vec![vec![]],
                },
                Transitions {
                    transitions: vec![4],
                    label: vec![vec![]],
                },
            ],
            initial_states: vec![0],
            acceptance_condition: AcceptanceCondition { states: vec![2, 4] },
            new_to_gnba: vec![(0, 0, 0), (1, 0, 1), (2, 0, 2), (3, 0, 3), (4, 0, 4)],
            gnba_to_new: FxHashMap::from_iter([
                ((0usize, 0usize), 0usize),
                ((1usize, 0usize), 1usize),
                ((2usize, 0usize), 2usize),
                ((3usize, 0usize), 3usize),
                ((4usize, 0usize), 4usize),
            ]),
            gnba: dummy_gnba,
        };

        nba.remove_dead_states();

        assert_eq!(nba.states.len(), 3);
        assert_eq!(nba.initial_states, vec![0]);
        assert_eq!(nba.acceptance_condition.states, vec![2]);
        assert_eq!(nba.transitions[0].transitions, vec![1]);
        assert_eq!(nba.transitions[1].transitions, vec![2]);
        assert_eq!(nba.transitions[2].transitions, vec![2]);
        assert!(nba.states.iter().all(|state| state.id < nba.states.len()));
    }

    #[test]
    fn test_nba_spot_equivalent() {
        use std::fs;
        use std::path::Path;
        use std::process::Command;

        // Spot installation path
        let spot_bin =
            std::env::var("SPOT_PATH").unwrap_or_else(|_| "./spot-2.15.1/bin".to_string());

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

            fs::write(&our_hoa, &hoa_content).expect("Failed to write NBA HOA file");

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

            fs::write(&ref_hoa, &output.stdout).expect("Failed to write reference HOA file");

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
            LTL::And(left, right) => format!(
                "({} & {})",
                formula_to_spot_ltl(left),
                formula_to_spot_ltl(right)
            ),
            LTL::Or(left, right) => format!(
                "({} | {})",
                formula_to_spot_ltl(left),
                formula_to_spot_ltl(right)
            ),
            LTL::Next(inner) => format!("X ({})", formula_to_spot_ltl(inner)),
            LTL::Eventually(inner) => format!("F ({})", formula_to_spot_ltl(inner)),
            LTL::Globally(inner) => format!("G ({})", formula_to_spot_ltl(inner)),
            LTL::Until(left, right) => format!(
                "({} U {})",
                formula_to_spot_ltl(left),
                formula_to_spot_ltl(right)
            ),
            LTL::Release(left, right) => format!(
                "({} R {})",
                formula_to_spot_ltl(left),
                formula_to_spot_ltl(right)
            ),
            LTL::WeakUntil(left, right) => format!(
                "({} W {})",
                formula_to_spot_ltl(left),
                formula_to_spot_ltl(right)
            ),
            LTL::TokenCount(_)
            | LTL::Fireable(_)
            | LTL::LessEqual(_, _)
            | LTL::GreaterEqual(_, _)
            | LTL::Greater(_, _)
            | LTL::Less(_, _) => "1".to_string(),
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
            1 => LTL::And(
                Box::new(random_ltl(depth - 1)),
                Box::new(random_ltl(depth - 1)),
            ),
            2 => LTL::Or(
                Box::new(random_ltl(depth - 1)),
                Box::new(random_ltl(depth - 1)),
            ),
            3 => LTL::Or(
                Box::new(LTL::Not(Box::new(random_ltl(depth - 1)))),
                Box::new(random_ltl(depth - 1)),
            ),
            4 => LTL::Next(Box::new(random_ltl(depth - 1))),
            5 => LTL::Eventually(Box::new(random_ltl(depth - 1))),
            6 => LTL::Globally(Box::new(random_ltl(depth - 1))),
            7 => LTL::Until(
                Box::new(random_ltl(depth - 1)),
                Box::new(random_ltl(depth - 1)),
            ),
            8 => LTL::Release(
                Box::new(random_ltl(depth - 1)),
                Box::new(random_ltl(depth - 1)),
            ),
            _ => LTL::WeakUntil(
                Box::new(random_ltl(depth - 1)),
                Box::new(random_ltl(depth - 1)),
            ),
        }
    }
}
