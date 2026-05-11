use crate::emptyness::check_emptyness_nba;
use crate::ltl_parser::LTL;
use crate::nba::NBA;
use crate::petri_net::{PetriNet, PetriState};
use std::collections::HashSet;

pub fn model_check(petri_net: &PetriNet, ltl: &LTL) -> bool {
    let negated_ltl = ltl.negate();
    let nba = NBA::new(&negated_ltl);
    let is_empty = check_emptyness_nba(&nba);
    if is_empty {
        println!(
            "The language of the NBA is empty, which means the original LTL formula is valid on all traces of the Petri net."
        );
        return true;
    }

    let has_counterexample = ndfs(petri_net, &nba);
    if has_counterexample {
        println!("Counterexample found: the property does NOT hold on the Petri net.");
        false
    } else {
        println!("No counterexample found in the product; the property holds.");
        true
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
struct CombinedState {
    petri_state: PetriState,
    nba_state: usize,
}

struct NDFSContext<'a> {
    petri: &'a PetriNet,
    nba: &'a NBA,

    seed: Option<(CombinedState, usize)>,

    visited: HashSet<(CombinedState, usize)>,
    stack: HashSet<CombinedState>,
    stack2: HashSet<CombinedState>,
}

impl<'a> NDFSContext<'a> {
    fn successors(&self, state: &CombinedState) -> Vec<CombinedState> {
        let mut result = Vec::new();

        let enabled = self.petri.next_states(&state.petri_state);

        for next_marking in enabled {
            let label = compute_label(&next_marking, self.petri, self.nba);

            let next_nba_states = self.nba.next(state.nba_state, &label);

            for q_next in next_nba_states {
                result.push(CombinedState {
                    petri_state: next_marking.clone(),
                    nba_state: q_next,
                });
            }
        }
        result
    }

    fn is_accepting(&self, state: &CombinedState) -> bool {
        self.nba.is_accepting(state.nba_state)
    }
}

fn compute_label(marking: &PetriState, petri: &PetriNet, nba: &NBA) -> Vec<bool> {
    fn eval_num(expr: &LTL, petri: &PetriNet, marking: &PetriState) -> Option<i64> {
        match expr {
            LTL::Number(n) => Some(*n as i64),
            LTL::TokenCount(name) => {
                if let Some(idx) = petri.places.iter().position(|p| &p.id == name) {
                    Some(marking.tokens.get(idx).copied().unwrap_or(0) as i64)
                } else {
                    Some(0)
                }
            }
            _ => None,
        }
    }

    fn eval_bool(formula: &LTL, petri: &PetriNet, marking: &PetriState) -> Option<bool> {
        match formula {
            LTL::True => Some(true),
            LTL::False => Some(false),
            LTL::Var(name) => petri
                .places
                .iter()
                .position(|p| &p.id == name)
                .map(|idx| marking.tokens.get(idx).copied().unwrap_or(0) > 0),
            LTL::Fireable(name) => {
                if let Some(trans) = petri.transitions.iter().find(|t| &t.id == name) {
                    Some(trans.is_fireable_tokens(&marking.tokens))
                } else {
                    Some(false)
                }
            }
            LTL::LessEqual(left, right) => {
                if let (Some(l), Some(r)) = (
                    eval_num(left, petri, marking),
                    eval_num(right, petri, marking),
                ) {
                    Some(l <= r)
                } else {
                    None
                }
            }
            LTL::GreaterEqual(left, right) => {
                if let (Some(l), Some(r)) = (
                    eval_num(left, petri, marking),
                    eval_num(right, petri, marking),
                ) {
                    Some(l >= r)
                } else {
                    None
                }
            }
            LTL::Greater(left, right) => {
                if let (Some(l), Some(r)) = (
                    eval_num(left, petri, marking),
                    eval_num(right, petri, marking),
                ) {
                    Some(l > r)
                } else {
                    None
                }
            }
            LTL::Less(left, right) => {
                if let (Some(l), Some(r)) = (
                    eval_num(left, petri, marking),
                    eval_num(right, petri, marking),
                ) {
                    Some(l < r)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    nba.closure
        .iter()
        .filter_map(|formula| match formula {
            LTL::Var(_)
            | LTL::True
            | LTL::False
            | LTL::LessEqual(_, _)
            | LTL::GreaterEqual(_, _)
            | LTL::Greater(_, _)
            | LTL::Less(_, _) => eval_bool(formula, petri, marking),
            LTL::Fireable(_) => eval_bool(formula, petri, marking),
            _ => None,
        })
        .collect()
}

fn ndfs(petri: &PetriNet, nba: &NBA) -> bool {
    let mut ctx = NDFSContext {
        petri,
        nba,
        seed: None,
        visited: HashSet::new(),
        stack: HashSet::new(),
        stack2: HashSet::new(),
    };

    let initial_marking = petri.initial_state();
    let initial_label = compute_label(&initial_marking, petri, nba);

    for q0 in nba.initial_states() {
        for q_start in nba.next(q0, &initial_label) {
            let init = CombinedState {
                petri_state: initial_marking.clone(),
                nba_state: q_start,
            };

            if dfs1(&mut ctx, init) {
                return true;
            }
        }
    }

    false
}

fn dfs1(ctx: &mut NDFSContext, init_state: CombinedState) -> bool {
    let mut call_stack = Vec::new();
    
    ctx.visited.insert((init_state.clone(), 0));
    ctx.stack.insert(init_state.clone());
    call_stack.push((init_state.clone(), ctx.successors(&init_state).into_iter()));

    while let Some((state, mut succs)) = call_stack.pop() {
        if let Some(succ) = succs.next() {
            call_stack.push((state.clone(), succs));
            
            if !ctx.visited.contains(&(succ.clone(), 0)) {
                ctx.visited.insert((succ.clone(), 0));
                ctx.stack.insert(succ.clone());
                call_stack.push((succ.clone(), ctx.successors(&succ).into_iter()));
            }
        } else {
            if ctx.is_accepting(&state) {
                ctx.seed = Some((state.clone(), 1));
                if dfs2(ctx, state.clone()) {
                    return true;
                }
            }
            ctx.stack.remove(&state);
        }
    }
    false
}

fn dfs2(ctx: &mut NDFSContext, init_state: CombinedState) -> bool {
    let mut call_stack = Vec::new();
    
    ctx.visited.insert((init_state.clone(), 1));
    ctx.stack2.insert(init_state.clone());
    call_stack.push((init_state.clone(), ctx.successors(&init_state).into_iter()));

    while let Some((state, mut succs)) = call_stack.pop() {
        if let Some(succ) = succs.next() {
            call_stack.push((state.clone(), succs));
            
            if ctx.seed == Some((succ.clone(), 1)) {
                return true;
            }
            if !ctx.visited.contains(&(succ.clone(), 1)) {
                ctx.visited.insert((succ.clone(), 1));
                ctx.stack2.insert(succ.clone());
                call_stack.push((succ.clone(), ctx.successors(&succ).into_iter()));
            }
        } else {
            ctx.stack2.remove(&state);
        }
    }
    false
}
