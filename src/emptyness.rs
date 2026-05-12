use crate::gnba::GNBA;
use crate::ltl_parser::LTL;
use crate::nba::NBA;

pub fn is_satisfiable(
    ltl: &LTL,
) -> (
    (bool, Option<Vec<usize>>, Option<Vec<usize>>),
    (bool, Option<Vec<usize>>, Option<Vec<usize>>),
) {
    let gnba = GNBA::new(ltl);
    let nba = NBA::new(ltl);
    let nba_empty = check_emptyness_nba(&nba);
    let gnba_empty = check_emptyness_gnba(&gnba);
    (
        (!nba_empty.0, nba_empty.1, nba_empty.2),
        (!gnba_empty.0, gnba_empty.1, gnba_empty.2),
    )
}

pub fn check_emptyness_nba(nba: &NBA) -> (bool, Option<Vec<usize>>, Option<Vec<usize>>) {
    for initial_state in &nba.initial_states {
        let visited = std::collections::HashSet::new();
        let stack = Vec::new();
        let stack2 = Vec::new();
        let mut ctx = NDFSContextNBA {
            nba,
            visited,
            stack,
            stack2,
            seed: None,
        };
        if dfs1_nba(&mut ctx, *initial_state) {
            return (false, Some(ctx.stack), Some(ctx.stack2));
        }
    }
    (true, None, None)
}

pub fn check_emptyness_gnba(gnba: &GNBA) -> (bool, Option<Vec<usize>>, Option<Vec<usize>>) {
    for initial_state in &gnba.initial_states {
        let visited = std::collections::HashSet::new();
        let stack = Vec::new();
        let stack2 = Vec::new();
        let mut ctx = NDFSContextGNBA {
            gnba,
            visited,
            stack,
            stack2,
            seed: None,
        };
        if dfs1_gnba(&mut ctx, *initial_state) {
            return (false, Some(ctx.stack), Some(ctx.stack2));
        }
    }
    (true, None, None)
}

struct NDFSContextNBA<'a> {
    nba: &'a NBA,
    visited: std::collections::HashSet<(usize, usize)>,
    stack: Vec<usize>,
    stack2: Vec<usize>,
    seed: Option<(usize, usize)>,
}

fn dfs1_nba(ctx: &mut NDFSContextNBA, state: usize) -> bool {
    ctx.visited.insert((state, 0));
    ctx.stack.push(state);

    for succ in ctx.nba.successors(state) {
        if !ctx.visited.contains(&(succ, 0)) && dfs1_nba(ctx, succ) {
            return true;
        }
    }

    if ctx.nba.is_accepting(state) {
        ctx.seed = Some((state, 1));
        if dfs2_nba(ctx, state) {
            return true;
        }
    }

    ctx.stack.pop();
    false
}

fn dfs2_nba(ctx: &mut NDFSContextNBA, state: usize) -> bool {
    ctx.visited.insert((state, 1));
    ctx.stack2.push(state);

    for succ in ctx.nba.successors(state) {
        if ctx.seed == Some((succ, 1)) {
            return true;
        }
        if !ctx.visited.contains(&(succ, 1)) && dfs2_nba(ctx, succ) {
            return true;
        }
    }

    ctx.stack2.pop();
    false
}

struct NDFSContextGNBA<'a> {
    gnba: &'a GNBA,
    visited: std::collections::HashSet<(usize, usize)>,
    stack: Vec<usize>,
    stack2: Vec<usize>,
    seed: Option<(usize, usize)>,
}

fn dfs1_gnba(ctx: &mut NDFSContextGNBA, state: usize) -> bool {
    ctx.visited.insert((state, 0));
    ctx.stack.push(state);

    for succ in ctx.gnba.successors(state) {
        if !ctx.visited.contains(&(succ, 0)) && dfs1_gnba(ctx, succ) {
            return true;
        }
    }

    if ctx.gnba.is_accepting(state) {
        ctx.seed = Some((state, 1));
        if dfs2_gnba(ctx, state) {
            return true;
        }
    }

    ctx.stack.pop();
    false
}

fn dfs2_gnba(ctx: &mut NDFSContextGNBA, state: usize) -> bool {
    ctx.visited.insert((state, 1));
    ctx.stack2.push(state);

    for succ in ctx.gnba.successors(state) {
        if ctx.seed == Some((succ, 1)) {
            return true;
        }
        if !ctx.visited.contains(&(succ, 1)) && dfs2_gnba(ctx, succ) {
            return true;
        }
    }

    ctx.stack2.pop();
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gnba::GNBA;
    use crate::ltl_parser::parse_ltl;
    use crate::nba::NBA;

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
            let (nba_sat, gnba_sat) = is_satisfiable(&ltl);

            assert_eq!(nba_sat.0, !expected_empty, "NBA mismatch for {formula}");
            assert_eq!(gnba_sat.0, !expected_empty, "GNBA mismatch for {formula}");

            let gnba = GNBA::new(&ltl);
            let nba = NBA::new(&ltl);

            assert_eq!(
                check_emptyness_gnba(&gnba).0,
                expected_empty,
                "GNBA emptiness mismatch for {formula}"
            );
            assert_eq!(
                check_emptyness_nba(&nba).0,
                expected_empty,
                "NBA emptiness mismatch for {formula}"
            );
        }
    }
}
