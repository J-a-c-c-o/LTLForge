use std::collections::HashSet;

use crate::ltl_parser::LTL;

pub fn compute_closure(ltl: &LTL) -> Vec<LTL> {
    let mut closure = Vec::new();
    let mut visited = HashSet::new();
    compute_closure_helper(ltl, &mut closure, &mut visited);
    sort_closure(&mut closure);
    closure
}

fn compute_closure_helper(ltl: &LTL, closure: &mut Vec<LTL>, visited: &mut HashSet<LTL>) {
    let non_negated = remove_negate(ltl);

    if !visited.insert(non_negated.clone()) {
        return;
    }

    push_unique(closure, non_negated.clone());

    match &non_negated {
        LTL::Not(inner) => compute_closure_helper(inner, closure, visited),
        LTL::And(left, right)
        | LTL::Or(left, right)
        | LTL::Implies(left, right)
        | LTL::Until(left, right)
        | LTL::Release(left, right)
        | LTL::WeakUntil(left, right)
        | LTL::MightyRelease(left, right) => {
            compute_closure_helper(left, closure, visited);
            compute_closure_helper(right, closure, visited);
        }
        LTL::Next(inner)
        | LTL::Eventually(inner)
        | LTL::Globally(inner)
        | LTL::AllPaths(inner)
        | LTL::SomePath(inner) => compute_closure_helper(inner, closure, visited),
        _ => {}
    }
}

fn push_unique(closure: &mut Vec<LTL>, formula: LTL) {
    if !closure.contains(&formula) {
        closure.push(formula);
    }
}

fn remove_negate(ltl: &LTL) -> LTL {
    match ltl {
        LTL::Not(inner) => inner.as_ref().clone(),
        _ => ltl.clone(),
    }
}

fn sort_closure(closure: &mut Vec<LTL>) {
    closure.sort_by(|a, b| a.size().cmp(&b.size()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closure() {
        let ltl = LTL::Not(Box::new(LTL::Eventually(Box::new(LTL::And(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::Eventually(Box::new(LTL::Var("b".to_string())))),
        )))));
        let closure = compute_closure(&ltl);

        let expected_closure = vec![
            LTL::Eventually(Box::new(LTL::And(
                Box::new(LTL::Var("a".to_string())),
                Box::new(LTL::Eventually(Box::new(LTL::Var("b".to_string())))),
            ))),
            LTL::And(
                Box::new(LTL::Var("a".to_string())),
                Box::new(LTL::Eventually(Box::new(LTL::Var("b".to_string())))),
            ),
            LTL::Var("a".to_string()),
            LTL::Eventually(Box::new(LTL::Var("b".to_string()))),
            LTL::Var("b".to_string()),
        ];

        assert_eq!(closure.len(), expected_closure.len());
        for expr in expected_closure {
            assert!(closure.contains(&expr));
        }
    }

    #[test]
    fn test_closure_until() {
        let ltl = LTL::Until(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::And(
                Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
                Box::new(LTL::Var("b".to_string())),
            )),
        );
        let closure = compute_closure(&ltl);

        let expected_closure = vec![
            LTL::Until(
                Box::new(LTL::Var("a".to_string())),
                Box::new(LTL::And(
                    Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
                    Box::new(LTL::Var("b".to_string())),
                )),
            ),
            LTL::Var("a".to_string()),
            LTL::And(
                Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
                Box::new(LTL::Var("b".to_string())),
            ),
            LTL::Var("b".to_string()),
        ];

        assert_eq!(closure.len(), expected_closure.len());
        for expr in expected_closure {
            assert!(closure.contains(&expr));
        }
    }
}
