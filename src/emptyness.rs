use crate::gnba::GNBA;
use crate::nba::NBA;
use crate::ltl_parser::LTL;

pub fn is_satisfiable(ltl: &LTL) -> (bool, bool) {
    let gnba = GNBA::new(ltl);
    let nba = NBA::new(ltl);
    let nba_empty = check_emptyness_nba(&nba);
    let gnba_empty = check_emptyness_gnba(&gnba);
    (!nba_empty, !gnba_empty)
}

pub fn check_emptyness_nba(nba: &NBA) -> bool {
    for initial_state in &nba.initial_states {
        let mut visited = vec![false; nba.states.len()];
        if dfs_nba(nba, *initial_state, &mut visited) {
            return false;
        }
    }
    true
}


pub fn check_emptyness_gnba(gnba: &GNBA) -> bool {
    for initial_state in &gnba.initial_states {
        let mut visited = vec![false; gnba.states.len()];
        if dfs_gnba(gnba, *initial_state, &mut visited) {
            return false;
        }
    }
    true
}

fn dfs_nba(nba: &NBA, current_state: usize, visited: &mut Vec<bool>) -> bool {
    if visited[current_state] {
        return false; // Already visited this state
    }
    visited[current_state] = true;

    if nba.acceptance_condition.states.contains(&current_state) {
        return true;
    }

    for transition in &nba.transitions {
        if transition.from == current_state {
            if dfs_nba(nba, transition.to, visited) {
                return true;
            }
        }
    }
    false
}

fn dfs_gnba(gnba: &GNBA, current_state: usize, visited: &mut Vec<bool>) -> bool {
    if visited[current_state] {
        return false;
    }
    visited[current_state] = true;

    for acc_condition in &gnba.acceptance_conditions {
        if acc_condition.states.contains(&current_state) {
            return true;
        }
    }

    for transition in &gnba.transitions {
        if transition.from == current_state {
            if dfs_gnba(gnba, transition.to, visited) {
                return true;
            }
        }
    }
    false
}