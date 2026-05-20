use crate::gnba::GNBA;
use crate::ltl_parser::LTL;
use crate::nba::NBA;
use crate::petri_net::{PetriNet, PetriState};

use rustc_hash::FxHashMap;
use std::fs;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StateColor {
    White,
    Cyan,
    Red,
    Blue,
}

pub fn model_check(
    petri_net: &PetriNet,
    ltl: &LTL,
    config: &ModelCheckConfig,
) -> Result<(bool, Option<Vec<CombinedState>>, Option<Vec<CombinedState>>), ModelCheckError> {
    let negated_ltl = ltl.negate();
    let nba = NBA::new(&negated_ltl);
    let is_empty = check_emptyness_generic(&nba, config)?;
    if is_empty.0 {
        return Ok((true, None, None));
    }

    let has_counterexample = ndfs_model_check(petri_net, &nba, config)?;
    Ok((
        !has_counterexample.0,
        has_counterexample.1,
        has_counterexample.2,
    ))
}


pub fn is_satisfiable(
    ltl: &LTL,
    config: &ModelCheckConfig,
) -> Result<(
    (bool, Option<Vec<usize>>, Option<Vec<usize>>),
    (bool, Option<Vec<usize>>, Option<Vec<usize>>),
), ModelCheckError> {
    let gnba = GNBA::new(ltl);
    let nba = NBA::new(ltl);
    let nba_empty = check_emptyness_generic(&nba, config)?;
    let gnba_empty = check_emptyness_generic(&gnba, config)?;
    Ok((
        (!nba_empty.0, nba_empty.1, nba_empty.2),
        (!gnba_empty.0, gnba_empty.1, gnba_empty.2),
    ))
}


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

impl std::fmt::Display for ModelCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelCheckError::Timeout => write!(f, "timed out"),
            ModelCheckError::MemoryLimitExceeded => write!(f, "memory limit exceeded"),
            ModelCheckError::CouldNotDetermineMemoryUsage => {
                write!(f, "could not determine memory usage")
            }
        }
    }
}

impl std::error::Error for ModelCheckError {}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct CombinedState {
    pub petri_state: PetriState,
    pub nba_state: usize,
}

trait NdfsEngine {
    type State: Clone + Eq + std::hash::Hash;
    type Error;

    fn check_limits(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn successors(&mut self, state: &Self::State) -> Result<Vec<Self::State>, Self::Error>;
    fn is_accepting(&self, state: &Self::State) -> bool;
}

struct NdfsContext<'a, P: NdfsEngine> {
    colors: FxHashMap<P::State, StateColor>,
    stack: Vec<P::State>,
    stack2: Vec<P::State>,
    problem: &'a mut P,
}

impl<'a, P: NdfsEngine> NdfsContext<'a, P> {
    fn new(problem: &'a mut P) -> Self {
        Self {
            colors: FxHashMap::default(),
            stack: Vec::new(),
            stack2: Vec::new(),
            problem,
        }
    }

    fn get_color(&self, s: &P::State) -> StateColor {
        *self.colors.get(s).unwrap_or(&StateColor::White)
    }

    fn set_color(&mut self, s: P::State, c: StateColor) {
        self.colors.insert(s, c);
    }
}

fn run_ndfs<P>(
    problem: &mut P,
    roots: Vec<P::State>,
) -> Result<(bool, Option<Vec<P::State>>, Option<Vec<P::State>>), P::Error>
where
    P: NdfsEngine,
{
    let mut ctx = NdfsContext::new(problem);
    for init in roots {
        if dfs_blue(&mut ctx, init)? {
            return Ok((true, Some(ctx.stack.clone()), Some(ctx.stack2.clone())));
        }
    }
    Ok((false, None, None))
}

fn dfs_blue<P>(ctx: &mut NdfsContext<'_, P>, s: P::State) -> Result<bool, P::Error>
where
    P: NdfsEngine,
{
    ctx.problem.check_limits()?;
    ctx.set_color(s.clone(), StateColor::Cyan);
    ctx.stack.push(s.clone());

    let succs = ctx.problem.successors(&s)?;
    for t in succs {
        if ctx.get_color(&t) == StateColor::White {
            if dfs_blue(ctx, t)? {
                return Ok(true);
            }
        }
    }

    if ctx.problem.is_accepting(&s) {
        if dfs_red(ctx, s.clone())? {
            return Ok(true);
        }
        ctx.set_color(s, StateColor::Red);
    } else {
        ctx.set_color(s, StateColor::Blue);
    }

    ctx.stack.pop();
    Ok(false)
}

fn dfs_red<P>(ctx: &mut NdfsContext<'_, P>, s: P::State) -> Result<bool, P::Error>
where
    P: NdfsEngine,
{
    ctx.problem.check_limits()?;
    ctx.stack2.push(s.clone());

    let succs = ctx.problem.successors(&s)?;
    for t in succs {
        match ctx.get_color(&t) {
            StateColor::Cyan => return Ok(true),
            StateColor::Blue => {
                ctx.set_color(t.clone(), StateColor::Red);
                if dfs_red(ctx, t)? {
                    return Ok(true);
                }
            }
            _ => {}
        }
    }

    ctx.stack2.pop();
    ctx.set_color(s, StateColor::Red);
    Ok(false)
}

trait Automaton {
    fn initial_states(&self) -> Vec<usize>;
    fn successors(&self, state: usize) -> &[usize];
    fn is_accepting(&self, state: usize) -> bool;
}

impl Automaton for NBA {
    fn initial_states(&self) -> Vec<usize> {
        self.initial_states()
    }

    fn successors(&self, state: usize) -> &[usize] {
        self.successors(state)
    }

    fn is_accepting(&self, state: usize) -> bool {
        self.is_accepting(state)
    }
}

impl Automaton for GNBA {
    fn initial_states(&self) -> Vec<usize> {
        self.initial_states.clone()
    }

    fn successors(&self, state: usize) -> &[usize] {
        self.successors(state)
    }

    fn is_accepting(&self, state: usize) -> bool {
        self.is_accepting(state)
    }
}

struct AutomatonNdfs<'a, A: Automaton> {
    automaton: &'a A,
    config: ModelCheckConfig,
    start_time: Instant,
}

impl<'a, A: Automaton> AutomatonNdfs<'a, A> {
    fn new(automaton: &'a A, config: &ModelCheckConfig) -> Self {
        Self {
            automaton,
            config: config.clone(),
            start_time: Instant::now(),
        }
    }
}

impl<'a, A: Automaton> NdfsEngine for AutomatonNdfs<'a, A> {
    type State = usize;
    type Error = ModelCheckError;

    fn check_limits(&mut self) -> Result<(), Self::Error> {
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

    fn successors(&mut self, state: &Self::State) -> Result<Vec<Self::State>, Self::Error> {
        Ok(self.automaton.successors(*state).to_vec())
    }

    fn is_accepting(&self, state: &Self::State) -> bool {
        self.automaton.is_accepting(*state)
    }
}

fn check_emptyness_generic<A: Automaton>(
    automaton: &A,
    config: &ModelCheckConfig,
) -> Result<(bool, Option<Vec<usize>>, Option<Vec<usize>>), ModelCheckError> {
    let roots = automaton.initial_states();
    let mut problem = AutomatonNdfs::new(automaton, config);
    let (found, stack, stack2) = run_ndfs(&mut problem, roots)?;

    if found {
        Ok((false, stack, stack2))
    } else {
        Ok((true, None, None))
    }
}

struct ProductNdfs<'a> {
    petri: &'a PetriNet,
    nba: &'a NBA,

    petri_successor_cache: FxHashMap<PetriState, Vec<PetriState>>,
    label_cache: FxHashMap<PetriState, Vec<bool>>,

    config: ModelCheckConfig,
    start_time: Instant,
}

impl<'a> ProductNdfs<'a> {
    fn new(petri: &'a PetriNet, nba: &'a NBA, config: &ModelCheckConfig) -> Self {
        Self {
            petri,
            nba,
            petri_successor_cache: FxHashMap::default(),
            label_cache: FxHashMap::default(),
            config: config.clone(),
            start_time: Instant::now(),
        }
    }

    fn petri_successors_cached(&mut self, state: &PetriState) -> Vec<PetriState> {
        if let Some(successors) = self.petri_successor_cache.get(state) {
            return successors.clone();
        }

        let successors = self.petri.next_states(state);
        self.petri_successor_cache
            .insert(state.clone(), successors.clone());
        successors
    }

    fn compute_label_cached(&mut self, marking: &PetriState) -> Vec<bool> {
        if let Some(label) = self.label_cache.get(marking) {
            return label.clone();
        }

        let label = compute_label(marking, self.petri, self.nba);
        self.label_cache.insert(marking.clone(), label.clone());
        label
    }
}

impl<'a> NdfsEngine for ProductNdfs<'a> {
    type State = CombinedState;
    type Error = ModelCheckError;

    fn check_limits(&mut self) -> Result<(), Self::Error> {
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

    fn successors(&mut self, state: &Self::State) -> Result<Vec<Self::State>, Self::Error> {
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

        Ok(result)
    }

    fn is_accepting(&self, state: &Self::State) -> bool {
        self.nba.is_accepting(state.nba_state)
    }
}

fn model_check_initial_roots(petri: &PetriNet, nba: &NBA) -> Vec<CombinedState> {
    let initial_marking = petri.initial_state();
    let initial_label = compute_label(&initial_marking, petri, nba);
    let mut roots = Vec::new();

    for q0 in nba.initial_states() {
        for q_start in nba.next(q0, &initial_label) {
            roots.push(CombinedState {
                petri_state: initial_marking.clone(),
                nba_state: q_start,
            });
        }
    }

    roots
}

fn ndfs_model_check(
    petri: &PetriNet,
    nba: &NBA,
    config: &ModelCheckConfig,
) -> Result<(bool, Option<Vec<CombinedState>>, Option<Vec<CombinedState>>), ModelCheckError> {
    let mut automaton = ProductNdfs::new(petri, nba, config);
    let roots = model_check_initial_roots(petri, nba);
    let (found, stack, stack2) = run_ndfs(&mut automaton, roots)?;

    if found {
        Ok((true, stack, stack2))
    } else {
        Ok((false, None, None))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ltl_parser::parse_ltl;

    #[test]
    fn test_emptiness_on_known_formulas() {
        let cases = [
            ("F a", false),
            ("G (a -> F b)", false),
            ("a & !a", true),
            ("G false", true),
        ];

        for (formula, expected_empty) in cases {
            let (_, ltl) = parse_ltl(formula).unwrap();
            let (nba_sat, gnba_sat) = is_satisfiable(&ltl, &ModelCheckConfig::default()).unwrap();

            assert_eq!(nba_sat.0, !expected_empty, "NBA mismatch for {formula}");
            assert_eq!(gnba_sat.0, !expected_empty, "GNBA mismatch for {formula}");

            let gnba = GNBA::new(&ltl);
            let nba = NBA::new(&ltl);

            assert_eq!(check_emptyness_gnba(&gnba, &ModelCheckConfig::default()).unwrap().0, expected_empty, "GNBA emptiness mismatch for {formula}");
            assert_eq!(check_emptyness_nba(&nba, &ModelCheckConfig::default()).unwrap().0, expected_empty, "NBA emptiness mismatch for {formula}");
        }
    }
}
