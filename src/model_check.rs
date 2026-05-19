use crate::emptyness::check_emptyness_nba;
use crate::ltl_parser::LTL;
use crate::nba::NBA;
use crate::petri_net::{PetriNet, PetriState};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fs;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct ModelCheckConfig {
    pub timeout_secs: Option<u64>,
    pub memory_limit_mb: Option<u64>,
}

impl Default for ModelCheckConfig {
    fn default() -> Self {
        Self {
            timeout_secs: Some(300),
            memory_limit_mb: Some(4096),
        }
    }
}

impl ModelCheckConfig {
    pub fn with_limits(timeout_secs: u64, memory_limit_mb: u64) -> Self {
        Self {
            timeout_secs: Some(timeout_secs),
            memory_limit_mb: Some(memory_limit_mb),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ModelCheckError {
    Timeout,
    MemoryLimitExceeded,
    CouldNotDetermineMemoryUsage,
}

pub fn model_check(
    petri_net: &PetriNet,
    ltl: &LTL,
    config: &ModelCheckConfig,
) -> Result<(bool, Option<Vec<CombinedState>>, Option<Vec<CombinedState>>), ModelCheckError> {
    let negated_ltl = ltl.negate();
    let nba = NBA::new(&negated_ltl);
    let is_empty = check_emptyness_nba(&nba);
    if is_empty.0 {
        return Ok((true, None, None));
    }

    let has_counterexample = ndfs(petri_net, &nba, config)?;
    Ok((
        !has_counterexample.0,
        has_counterexample.1,
        has_counterexample.2,
    ))
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct CombinedState {
    pub petri_state: PetriState,
    pub nba_state: usize,
}

struct NDFSContext<'a> {
    petri: &'a PetriNet,
    nba: &'a NBA,

    petri_successor_cache: FxHashMap<PetriState, Vec<PetriState>>,
    label_cache: FxHashMap<PetriState, Vec<bool>>,

    seed: Option<(CombinedState, usize)>,

    visited: FxHashSet<(CombinedState, usize)>,
    stack: Vec<CombinedState>,
    stack2: Vec<CombinedState>,

    config: ModelCheckConfig,
    start_time: Instant,
}

impl<'a> NDFSContext<'a> {
    fn petri_successors_cached(&mut self, state: &PetriState) -> Vec<PetriState> {
        if let Some(successors) = self.petri_successor_cache.get(state) {
            return successors.clone();
        }

        let successors = self.petri.next_states(state);
        self.petri_successor_cache
            .insert(state.clone(), successors.clone());
        successors
    }

    fn check_limits(&self) -> Result<(), ModelCheckError> {
        if let Some(timeout_secs) = self.config.timeout_secs {
            let elapsed = self.start_time.elapsed();
            if elapsed > Duration::from_secs(timeout_secs) {
                return Err(ModelCheckError::Timeout);
            }
        }

        if let Some(memory_limit_mb) = self.config.memory_limit_mb {
            let memory_limit_bytes = memory_limit_mb.saturating_mul(1024 * 1024);
            if let Some(current_rss_bytes) = current_process_rss_bytes() {
                if current_rss_bytes > memory_limit_bytes {
                    return Err(ModelCheckError::MemoryLimitExceeded);
                }
            } else {
                return Err(ModelCheckError::CouldNotDetermineMemoryUsage);
            }
        }
        Ok(())
    }

    fn successors(&mut self, state: &CombinedState) -> Vec<CombinedState> {
        let mut result = Vec::new();

        let enabled = self.petri_successors_cached(&state.petri_state);

        for next_marking in enabled {
            let label = self.compute_label_cached(&next_marking);

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

    fn compute_label_cached(&mut self, marking: &PetriState) -> Vec<bool> {
        if let Some(label) = self.label_cache.get(marking) {
            return label.clone();
        }

        let label = compute_label(marking, self.petri, self.nba);
        self.label_cache.insert(marking.clone(), label.clone());
        label
    }

    fn is_accepting(&self, state: &CombinedState) -> bool {
        self.nba.is_accepting(state.nba_state)
    }
}

fn compute_label(marking: &PetriState, petri: &PetriNet, nba: &NBA) -> Vec<bool> {
    fn eval_num(expr: &LTL, petri: &PetriNet, marking: &PetriState) -> Option<i64> {
        match expr {
            LTL::Number(n) => Some(*n as i64),
            LTL::TokenCount(names) => {
                let total: i64 = names
                    .iter()
                    .filter_map(|name| {
                        petri
                            .places
                            .iter()
                            .position(|p| &p.id == name)
                            .map(|idx| marking.tokens.get(idx).copied().unwrap_or(0) as i64)
                    })
                    .sum();
                Some(total)
            }
            _ => None,
        }
    }

    fn eval_bool(formula: &LTL, petri: &PetriNet, marking: &PetriState) -> Option<bool> {
        match formula {
            LTL::Var(name) => petri
                .places
                .iter()
                .position(|p| &p.id == name)
                .map(|idx| marking.tokens.get(idx).copied().unwrap_or(0) > 0),
            LTL::Fireable(names) => Some(names.iter().any(|name| {
                petri
                    .transitions
                    .iter()
                    .find(|t| &t.id == name)
                    .is_some_and(|trans| trans.is_fireable_tokens(&marking.tokens))
            })),
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
            | LTL::LessEqual(_, _)
            | LTL::GreaterEqual(_, _)
            | LTL::Greater(_, _)
            | LTL::Less(_, _) => eval_bool(formula, petri, marking),
            LTL::Fireable(_) => eval_bool(formula, petri, marking),
            _ => None,
        })
        .collect()
}

fn ndfs(
    petri: &PetriNet,
    nba: &NBA,
    config: &ModelCheckConfig,
) -> Result<(bool, Option<Vec<CombinedState>>, Option<Vec<CombinedState>>), ModelCheckError> {
    let mut ctx = NDFSContext {
        petri,
        nba,
        petri_successor_cache: FxHashMap::default(),
        label_cache: FxHashMap::default(),
        seed: None,
        visited: FxHashSet::default(),
        stack: Vec::new(),
        stack2: Vec::new(),
        config: config.clone(),
        start_time: Instant::now(),
    };

    let initial_marking = petri.initial_state();
    let initial_label = compute_label(&initial_marking, petri, nba);

    for q0 in nba.initial_states() {
        for q_start in nba.next(q0, &initial_label) {
            let init = CombinedState {
                petri_state: initial_marking.clone(),
                nba_state: q_start,
            };

            if dfs1(&mut ctx, init)? {
                return Ok((true, Some(ctx.stack.clone()), Some(ctx.stack2.clone())));
            }
        }
    }

    Ok((false, None, None))
}

fn dfs1(ctx: &mut NDFSContext, init_state: CombinedState) -> Result<bool, ModelCheckError> {
    let mut call_stack = Vec::new();

    ctx.visited.insert((init_state.clone(), 0));
    ctx.stack.push(init_state.clone());
    call_stack.push((init_state.clone(), ctx.successors(&init_state).into_iter()));

    while let Some((state, mut succs)) = call_stack.pop() {
        ctx.check_limits()?;

        if let Some(succ) = succs.next() {
            call_stack.push((state.clone(), succs));

            if !ctx.visited.contains(&(succ.clone(), 0)) {
                ctx.visited.insert((succ.clone(), 0));
                ctx.stack.push(succ.clone());
                call_stack.push((succ.clone(), ctx.successors(&succ).into_iter()));
            }
        } else {
            if ctx.is_accepting(&state) {
                ctx.seed = Some((state.clone(), 1));
                if dfs2(ctx, state.clone())? {
                    return Ok(true);
                }
            }
            ctx.stack.pop();
        }
    }

    ctx.stack.clear();
    Ok(false)
}

fn dfs2(ctx: &mut NDFSContext, init_state: CombinedState) -> Result<bool, ModelCheckError> {
    let mut call_stack = Vec::new();

    ctx.visited.insert((init_state.clone(), 1));
    ctx.stack2.push(init_state.clone());
    call_stack.push((init_state.clone(), ctx.successors(&init_state).into_iter()));

    while let Some((state, mut succs)) = call_stack.pop() {
        ctx.check_limits()?;

        if let Some(succ) = succs.next() {
            call_stack.push((state.clone(), succs));

            if ctx.seed == Some((succ.clone(), 1)) {
                return Ok(true);
            }
            if !ctx.visited.contains(&(succ.clone(), 1)) {
                ctx.visited.insert((succ.clone(), 1));
                ctx.stack2.push(succ.clone());
                call_stack.push((succ.clone(), ctx.successors(&succ).into_iter()));
            }
        } else {
            ctx.stack2.pop();
        }
    }

    ctx.stack2.clear();
    Ok(false)
}

fn current_process_rss_bytes() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    parse_vm_rss_bytes(&status)
}

fn parse_vm_rss_bytes(status: &str) -> Option<u64> {
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let rss_kb = rest.split_whitespace().next()?.parse::<u64>().ok()?;
            return Some(rss_kb * 1024);
        }
    }

    None
}

