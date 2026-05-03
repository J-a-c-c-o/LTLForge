use crate::ltl_parser::LTL;

pub fn to_pnf(ltl: &LTL) -> LTL {
    let mut pnf = ltl.clone();
    pnf_simplifications(&mut pnf);
    push_pnf_inwards(&mut pnf);
    pnf_simplifications(&mut pnf);
    pnf_eliminate_temporal_operators(&mut pnf);
    pnf_simplifications(&mut pnf);
    pnf
}

fn push_pnf_inwards(expr: &mut LTL) {
    match expr {
        LTL::Not(inner) => match &**inner {
            LTL::Next(inner) => {
                *expr = LTL::Next(Box::new(LTL::Not(inner.clone())));
                push_pnf_inwards(expr);
            }
            LTL::Globally(inner) => {
                *expr = LTL::Eventually(Box::new(LTL::Not(inner.clone())));
                push_pnf_inwards(expr);
            }
            LTL::Eventually(inner) => {
                *expr = LTL::Globally(Box::new(LTL::Not(inner.clone())));
                push_pnf_inwards(expr);
            }
            LTL::Until(left, right) => {
                *expr = LTL::Release(
                    Box::new(LTL::Not(left.clone())),
                    Box::new(LTL::Not(right.clone())),
                );
                push_pnf_inwards(expr);
            }
            LTL::Release(left, right) => {
                *expr = LTL::Until(
                    Box::new(LTL::Not(left.clone())),
                    Box::new(LTL::Not(right.clone())),
                );
                push_pnf_inwards(expr);
            }
            LTL::WeakUntil(left, right) => {
                *expr = LTL::MightyRelease(
                    Box::new(LTL::Not(left.clone())),
                    Box::new(LTL::Not(right.clone())),
                );
                push_pnf_inwards(expr);
            }
            LTL::MightyRelease(left, right) => {
                *expr = LTL::WeakUntil(
                    Box::new(LTL::Not(left.clone())),
                    Box::new(LTL::Not(right.clone())),
                );
                push_pnf_inwards(expr);
            }
            LTL::And(left, right) => {
                *expr = LTL::Or(
                    Box::new(LTL::Not(left.clone())),
                    Box::new(LTL::Not(right.clone())),
                );
                push_pnf_inwards(expr);
            }
            LTL::Or(left, right) => {
                *expr = LTL::And(
                    Box::new(LTL::Not(left.clone())),
                    Box::new(LTL::Not(right.clone())),
                );
                push_pnf_inwards(expr);
            }
            _ => {}
        },
        LTL::And(left, right)
        | LTL::Or(left, right)
        | LTL::Until(left, right)
        | LTL::Release(left, right)
        | LTL::Implies(left, right)
        | LTL::WeakUntil(left, right)
        | LTL::MightyRelease(left, right)
        | LTL::LessEqual(left, right)
        | LTL::GreaterEqual(left, right)
        | LTL::Greater(left, right)
        | LTL::Less(left, right) => {
            push_pnf_inwards(left);
            push_pnf_inwards(right);
        }
        LTL::Next(inner) | LTL::Globally(inner) | LTL::Eventually(inner) | LTL::AllPaths(inner) | LTL::SomePath(inner) => {
            push_pnf_inwards(inner);
        }
        _ => {}
    }
}

fn pnf_eliminate_temporal_operators(expr: &mut LTL) {
    match expr {
        LTL::Eventually(inner) => {
            *expr = LTL::Until(Box::new(LTL::True), Box::new(*inner.clone()));
            pnf_eliminate_temporal_operators(expr);
        }
        LTL::Globally(inner) => {
            *expr = LTL::Release(Box::new(LTL::False), Box::new(*inner.clone()));
            pnf_eliminate_temporal_operators(expr);
        }
        LTL::WeakUntil(left, right) => {
            *expr = LTL::Release(
                Box::new(*right.clone()),
                Box::new(LTL::Or(Box::new(*left.clone()), Box::new(*right.clone()))),
            );
            pnf_eliminate_temporal_operators(expr);
        }
        LTL::MightyRelease(left, right) => {
            *expr = LTL::Until(
                Box::new(*right.clone()),
                Box::new(LTL::And(Box::new(*left.clone()), Box::new(*right.clone()))),
            );
            pnf_eliminate_temporal_operators(expr);
        }
        _ => {}
    }
}

fn pnf_simplifications(expr: &mut LTL) {
    match expr {
        LTL::Globally(inner) => {
            if let LTL::Globally(inner_inner) = &**inner {
                *expr = LTL::Globally(inner_inner.clone());
                pnf_simplifications(expr);
            }
        }
        LTL::Eventually(inner) => {
            if let LTL::Eventually(inner_inner) = &**inner {
                *expr = LTL::Eventually(inner_inner.clone());
                pnf_simplifications(expr);
            }
        }
        LTL::Next(inner) => {
            if let LTL::Or(left, right) = &**inner {
                *expr = LTL::Or(
                    Box::new(LTL::Next(left.clone())),
                    Box::new(LTL::Next(right.clone())),
                );
                pnf_simplifications(expr);
            } else if let LTL::And(left, right) = &**inner {
                *expr = LTL::And(
                    Box::new(LTL::Next(left.clone())),
                    Box::new(LTL::Next(right.clone())),
                );
                pnf_simplifications(expr);
            } else if let LTL::Until(left, right) = &**inner {
                *expr = LTL::Until(
                    Box::new(LTL::Next(left.clone())),
                    Box::new(LTL::Next(right.clone())),
                );
                pnf_simplifications(expr);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_push_pnf_inwards_and() {
        let mut ltl = LTL::Not(Box::new(LTL::And(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::Var("b".to_string())),
        )));

        push_pnf_inwards(&mut ltl);

        let expected = LTL::Or(
            Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
            Box::new(LTL::Not(Box::new(LTL::Var("b".to_string())))),
        );

        assert_eq!(ltl, expected);
    }

    #[test]
    fn test_to_push_pnf_inwards_or() {
        let mut ltl = LTL::Not(Box::new(LTL::Or(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::Var("b".to_string())),
        )));

        push_pnf_inwards(&mut ltl);

        let expected = LTL::And(
            Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
            Box::new(LTL::Not(Box::new(LTL::Var("b".to_string())))),
        );

        assert_eq!(ltl, expected);
    }

    #[test]
    fn test_to_push_pnf_inwards_next() {
        let mut ltl = LTL::Not(Box::new(LTL::Next(Box::new(LTL::Var("a".to_string())))));

        push_pnf_inwards(&mut ltl);

        let expected = LTL::Next(Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))));

        assert_eq!(ltl, expected);
    }

    #[test]
    fn test_to_push_pnf_inwards_until() {
        let mut ltl = LTL::Not(Box::new(LTL::Until(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::Var("b".to_string())),
        )));

        push_pnf_inwards(&mut ltl);

        let expected = LTL::Release(
            Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
            Box::new(LTL::Not(Box::new(LTL::Var("b".to_string())))),
        );

        assert_eq!(ltl, expected);
    }

    #[test]
    fn test_to_pnf_eliminate_temporal_operators() {
        let mut ltl = LTL::Eventually(Box::new(LTL::Var("a".to_string())));

        pnf_eliminate_temporal_operators(&mut ltl);

        let expected = LTL::Until(Box::new(LTL::True), Box::new(LTL::Var("a".to_string())));

        assert_eq!(ltl, expected);
    }

    #[test]
    fn test_to_pnf_eliminate_temporal_operators_globally() {
        let mut ltl = LTL::Globally(Box::new(LTL::Var("a".to_string())));

        pnf_eliminate_temporal_operators(&mut ltl);

        let expected = LTL::Release(Box::new(LTL::False), Box::new(LTL::Var("a".to_string())));

        assert_eq!(ltl, expected);
    }

    #[test]
    fn test_to_pnf_simplifications() {
        let mut ltl = LTL::Globally(Box::new(LTL::Globally(Box::new(LTL::Var("a".to_string())))));

        pnf_simplifications(&mut ltl);

        let expected = LTL::Globally(Box::new(LTL::Var("a".to_string())));

        assert_eq!(ltl, expected);
    }

    #[test]
    fn test_to_push_pnf_full() {
        let ltl = LTL::Not(Box::new(LTL::Eventually(Box::new(LTL::And(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::Var("b".to_string())),
        )))));

        let pnf = to_pnf(&ltl);

        let expected = LTL::Release(
            Box::new(LTL::False),
            Box::new(LTL::Or(
                Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
                Box::new(LTL::Not(Box::new(LTL::Var("b".to_string())))),
            )),
        );

        assert_eq!(pnf, expected);
    }

    #[test]
    fn test_to_push_pnf_full_nested() {
        let ltl = LTL::Not(Box::new(LTL::Eventually(Box::new(LTL::And(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::Eventually(Box::new(LTL::Var("b".to_string())))),
        )))));

        let pnf = to_pnf(&ltl);

        let expected = LTL::Release(
            Box::new(LTL::False),
            Box::new(LTL::Or(
                Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
                Box::new(LTL::Globally(Box::new(LTL::Not(Box::new(LTL::Var("b".to_string())))))),
            )),
        );

        assert_eq!(pnf, expected);
    }
}
