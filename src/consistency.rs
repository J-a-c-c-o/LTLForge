use crate::ltl_parser::LTL;
use std::collections::HashSet;

pub fn is_consistent(closure: &[LTL]) -> Vec<Vec<bool>> {
    let mut hash_set: HashSet<Vec<bool>> = HashSet::new();
    consistent_sets(closure, &[], &mut hash_set);
    let consistent_sets: Vec<Vec<bool>> = hash_set.into_iter().collect();

    consistent_sets
}

fn consistent_sets(closure: &[LTL], partial_truth: &[bool], hash_set: &mut HashSet<Vec<bool>>) {
    if partial_truth.len() == closure.len() {
        hash_set.insert(partial_truth.to_vec());
        return;
    }

    let index = partial_truth.len();
    let formula = &closure[index];

    let mut new_truth = partial_truth.to_vec();
    new_truth.push(true);
    if is_consistent_with_partial(formula, &new_truth, closure) {
        consistent_sets(closure, &new_truth, hash_set);
    }

    let mut new_truth = partial_truth.to_vec();
    new_truth.push(false);
    if is_consistent_with_partial(formula, &new_truth, closure) {
        consistent_sets(closure, &new_truth, hash_set);
    }
}

fn find_index(closure: &[LTL], formula: &LTL) -> Option<usize> {
    closure.iter().position(|f| f == formula)
}

fn is_consistent_with_partial(formula: &LTL, partial_truth: &[bool], closure: &[LTL]) -> bool {
    let idx = partial_truth.len() - 1;
    let value = partial_truth[idx];

    match formula {
        LTL::True
            if !value => {
                return false;
            }
        LTL::False
            if value => {
                return false;
            }
        LTL::And(left, right) => {
            let left_assigned = get_assigned_truth(left.as_ref(), partial_truth, closure);
            let right_assigned = get_assigned_truth(right.as_ref(), partial_truth, closure);

            if value {
                if matches!(left_assigned, Some(false)) || matches!(right_assigned, Some(false)) {
                    return false;
                }
            } else {
                if matches!(left_assigned, Some(true)) && matches!(right_assigned, Some(true)) {
                    return false;
                }
            }
        }
        LTL::Or(left, right) => {
            let left_assigned = get_assigned_truth(left.as_ref(), partial_truth, closure);
            let right_assigned = get_assigned_truth(right.as_ref(), partial_truth, closure);

            if value {
                if matches!(left_assigned, Some(false)) && matches!(right_assigned, Some(false)) {
                    return false;
                }
            } else {
                if matches!(left_assigned, Some(true)) || matches!(right_assigned, Some(true)) {
                    return false;
                }
            }
        }
        LTL::Implies(left, right) => {
            let left_assigned = get_assigned_truth(left.as_ref(), partial_truth, closure);
            let right_assigned = get_assigned_truth(right.as_ref(), partial_truth, closure);

            if value {
                if matches!(left_assigned, Some(true)) && matches!(right_assigned, Some(false)) {
                    return false;
                }
            } else {
                if matches!(left_assigned, Some(false)) || matches!(right_assigned, Some(true)) {
                    return false;
                }
            }
        }
        LTL::Next(_) => {
            // Next refers to the next state; no local constraint enforced here
        }
        LTL::Until(left, right) => {
            let left_assigned = get_assigned_truth(left.as_ref(), partial_truth, closure);
            let right_assigned = get_assigned_truth(right.as_ref(), partial_truth, closure);

            if value {
                if matches!(left_assigned, Some(false)) && matches!(right_assigned, Some(false)) {
                    return false;
                }
            } else {
                if matches!(right_assigned, Some(true)) {
                    return false;
                }
            }
        }
        LTL::Release(left, right) => {
            let left_assigned = get_assigned_truth(left.as_ref(), partial_truth, closure);
            let right_assigned = get_assigned_truth(right.as_ref(), partial_truth, closure);

            if value {
                if matches!(right_assigned, Some(false)) {
                    return false;
                }
            } else {
                if matches!(left_assigned, Some(true)) && matches!(right_assigned, Some(true)) {
                    return false;
                }
            }
        }
        _ => {}
    }

    true
}

fn get_assigned_truth(formula: &LTL, partial_truth: &[bool], closure: &[LTL]) -> Option<bool> {
    match formula {
        LTL::Not(inner) => {
            if let Some(i) = find_index(closure, inner.as_ref())
                && i < partial_truth.len() {
                    return Some(!partial_truth[i]);
                }
            None
        }
        _ => {
            if let Some(i) = find_index(closure, formula)
                && i < partial_truth.len() {
                    return Some(partial_truth[i]);
                }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        let closure = vec![LTL::True];
        let sets = is_consistent(&closure);
        assert_eq!(sets.len(), 1);
        assert!(sets.contains(&vec![true]));

        let closure = vec![LTL::False];
        let sets = is_consistent(&closure);
        assert_eq!(sets.len(), 1);
        assert!(sets.contains(&vec![false]));
    }

    #[test]
    fn test_and_consistency() {
        // closure: a, b, (a & b)
        let a = LTL::Var("a".to_string());
        let b = LTL::Var("b".to_string());
        let and = LTL::And(Box::new(a.clone()), Box::new(b.clone()));
        let closure = vec![a.clone(), b.clone(), and.clone()];

        let sets = is_consistent(&closure);
        // expected: (a,b,and) where and == a & b -> 4 consistent sets
        let expected = vec![
            vec![false, false, false],
            vec![false, true, false],
            vec![true, false, false],
            vec![true, true, true],
        ];

        for e in expected {
            assert!(sets.contains(&e));
        }
        assert_eq!(sets.len(), 4);
    }

    #[test]
    fn test_implies_consistency() {
        // closure: a, b, (a -> b)
        let a = LTL::Var("a".to_string());
        let b = LTL::Var("b".to_string());
        let imp = LTL::Implies(Box::new(a.clone()), Box::new(b.clone()));
        let closure = vec![a.clone(), b.clone(), imp.clone()];

        let sets = is_consistent(&closure);

        // implication truth table: imp == (!a) || b
        let expected = vec![
            vec![false, false, true],
            vec![false, true, true],
            vec![true, false, false],
            vec![true, true, true],
        ];

        for e in expected {
            assert!(sets.contains(&e));
        }
        assert_eq!(sets.len(), 4);
    }

    #[test]
    fn test_until_local_rule() {
        // a U (¬a & b) closure components
        let a = LTL::Var("a".to_string());
        let not_a = LTL::Not(Box::new(a.clone()));
        let b = LTL::Var("b".to_string());
        let and = LTL::And(Box::new(not_a.clone()), Box::new(b.clone()));
        let until = LTL::Until(Box::new(a.clone()), Box::new(and.clone()));

        let closure = vec![a.clone(), b.clone(), and.clone(), until.clone()];

        let sets = is_consistent(&closure);

        println!("Consistent sets:");
        for s in &sets {
            println!("{:?}", s);
        }

        let consitentsy_sets = vec![
            vec![true, true, false, true],
            vec![true, true, false, false],
            vec![true, false, false, true],
            vec![true, false, false, false],
            vec![false, true, true, true],
            vec![false, false, false, false],
        ];

        for s in consitentsy_sets {
            assert!(sets.contains(&s));
        }
        assert_eq!(sets.len(), 6);
    }
}
