use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alpha1, alphanumeric1, char, digit1, multispace0},
    combinator::{map, value},
    multi::many0,
    sequence::{delimited, pair, preceded, terminated},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum Expr {
    True,
    False,
    Var(String),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Implies(Box<Expr>, Box<Expr>),
    LessEqual(Box<Expr>, Box<Expr>),
    GreaterEqual(Box<Expr>, Box<Expr>),
    TokenCount(String),
    Fireable(String),
    Number(u32),
}

fn parse_expr(input: &str) -> IResult<&str, Expr> {
    parse_expr_implies(input)
}

fn parse_expr_implies(input: &str) -> IResult<&str, Expr> {
    let (input, lhs) = parse_expr_or(input)?;

    if let Ok((next_input, rhs)) = preceded(ws(tag("->")), parse_expr_implies).parse(input) {
        Ok((next_input, Expr::Implies(Box::new(lhs), Box::new(rhs))))
    } else {
        Ok((input, lhs))
    }
}

fn parse_expr_or(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut expr) = parse_expr_and(input)?;

    while let Ok((next_input, rhs)) = preceded(ws(alt((tag("||"), tag("|")))), parse_expr_and)
        .parse(input)
    {
        expr = Expr::Or(Box::new(expr), Box::new(rhs));
        input = next_input;
    }

    Ok((input, expr))
}

fn parse_expr_and(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut expr) = parse_expr_compare(input)?;

    while let Ok((next_input, rhs)) = preceded(ws(alt((tag("&&"), tag("&")))), parse_expr_compare)
        .parse(input)
    {
        expr = Expr::And(Box::new(expr), Box::new(rhs));
        input = next_input;
    }

    Ok((input, expr))
}

fn parse_expr_compare(input: &str) -> IResult<&str, Expr> {
    let (input, lhs) = parse_expr_not(input)?;

    if let Ok((next_input, _)) = parse_le_operator(input) {
        let (next_input, rhs) = parse_expr_not(next_input)?;
        Ok((next_input, Expr::LessEqual(Box::new(lhs), Box::new(rhs))))
    } else if let Ok((next_input, _)) = parse_ge_operator(input) {
        let (next_input, rhs) = parse_expr_not(next_input)?;
        Ok((next_input, Expr::GreaterEqual(Box::new(lhs), Box::new(rhs))))
    } else {
        Ok((input, lhs))
    }
}

fn parse_le_operator(input: &str) -> IResult<&str, &str> {
    ws(tag("<=")).parse(input)
}

fn parse_ge_operator(input: &str) -> IResult<&str, &str> {
    ws(tag(">=")).parse(input)
}

fn parse_expr_not(input: &str) -> IResult<&str, Expr> {
    alt((
        map(preceded(ws(char('!')), parse_expr_not), |expr| {
            Expr::Not(Box::new(expr))
        }),
        parse_expr_primary,
    ))
    .parse(input)
}

fn parse_expr_primary(input: &str) -> IResult<&str, Expr> {
    alt((
        value(Expr::True, ws(tag("true"))),
        value(Expr::False, ws(tag("false"))),
        parse_token_count,
        parse_fireable,
        map(ws(digit1), |n: &str| Expr::Number(n.parse().unwrap())),
        parse_variable_expr,
        delimited(ws(char('(')), parse_expr, ws(char(')'))),
    ))
    .parse(input)
}

fn parse_variable_expr(input: &str) -> IResult<&str, Expr> {
    let (input, name) = ws(parse_identifier).parse(input)?;

    if matches!(name, "A" | "E" | "X" | "F" | "G" | "U" | "R" | "W" | "M") {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    Ok((input, Expr::Var(name.to_string())))
}

fn parse_token_count(input: &str) -> IResult<&str, Expr> {
    map(
        preceded(
            ws(tag("#tokens")),
            delimited(ws(char('(')), ws(parse_quoted_string), ws(char(')'))),
        ),
        |name| Expr::TokenCount(name.to_string()),
    )
    .parse(input)
}

fn parse_fireable(input: &str) -> IResult<&str, Expr> {
    map(terminated(ws(parse_quoted_string), ws(char('?'))), |name| {
        Expr::Fireable(name.to_string())
    })
    .parse(input)
}

fn parse_quoted_string(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), take_while(|c| c != '"'), char('"')).parse(input)
}

fn parse_identifier(input: &str) -> IResult<&str, &str> {
    nom::combinator::recognize(pair(
        alt((alpha1, tag("_"), tag("-"))),
        many0(alt((alphanumeric1, tag("_"), tag("-")))),
    ))
    .parse(input)
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum LTL {
    Prop(Expr),
    Not(Box<LTL>),
    And(Box<LTL>, Box<LTL>),
    Or(Box<LTL>, Box<LTL>),
    Implies(Box<LTL>, Box<LTL>),
    Next(Box<LTL>),
    Eventually(Box<LTL>),
    Globally(Box<LTL>),
    AllPaths(Box<LTL>),
    SomePath(Box<LTL>),
    Until(Box<LTL>, Box<LTL>),
    WeakUntil(Box<LTL>, Box<LTL>),
    Release(Box<LTL>, Box<LTL>),
    MightyRelease(Box<LTL>, Box<LTL>),
}

fn ws<'a, O, E, P>(parser: P) -> impl Parser<&'a str, Output = O, Error = E>
where
    E: nom::error::ParseError<&'a str>,
    P: Parser<&'a str, Output = O, Error = E>,
{
    delimited(multispace0, parser, multispace0)
}

fn parse_prop(input: &str) -> IResult<&str, LTL> {
    map(parse_expr, |expr| LTL::Prop(expr)).parse(input)
}


fn parse_primary(input: &str) -> IResult<&str, LTL> {
    alt((
        delimited(ws(char('(')), parse_ltl, ws(char(')'))),
        parse_prop,
    ))
    .parse(input)
}

fn parse_unary(input: &str) -> IResult<&str, LTL> {
    alt((
        map(preceded(ws(char('!')), parse_unary), |ltl| {
            LTL::Not(Box::new(ltl))
        }),
        map(preceded(ws(tag("A")), parse_unary), |ltl| {
            LTL::AllPaths(Box::new(ltl))
        }),
        map(preceded(ws(tag("E")), parse_unary), |ltl| {
            LTL::SomePath(Box::new(ltl))
        }),
        map(preceded(ws(char('X')), parse_unary), |ltl| {
            LTL::Next(Box::new(ltl))
        }),
        map(preceded(ws(char('F')), parse_unary), |ltl| {
            LTL::Eventually(Box::new(ltl))
        }),
        map(preceded(ws(char('G')), parse_unary), |ltl| {
            LTL::Globally(Box::new(ltl))
        }),
        parse_primary,
    ))
    .parse(input)
}

fn parse_and(input: &str) -> IResult<&str, LTL> {
    let (mut input, mut expr) = parse_unary(input)?;

    while let Ok((next_input, rhs)) = preceded(ws(alt((tag("&&"), tag("&")))), parse_unary)
        .parse(input)
    {
        expr = LTL::And(Box::new(expr), Box::new(rhs));
        input = next_input;
    }

    Ok((input, expr))
}

fn parse_or(input: &str) -> IResult<&str, LTL> {
    let (mut input, mut expr) = parse_and(input)?;

    while let Ok((next_input, rhs)) = preceded(ws(alt((tag("||"), tag("|")))), parse_and)
        .parse(input)
    {
        expr = LTL::Or(Box::new(expr), Box::new(rhs));
        input = next_input;
    }

    Ok((input, expr))
}


fn parse_until(input: &str) -> IResult<&str, LTL> {
    let (input, lhs) = parse_or(input)?;

    if let Ok((next_input, rhs)) = preceded(ws(tag("U")), parse_until).parse(input) {
        Ok((next_input, LTL::Until(Box::new(lhs), Box::new(rhs))))
    } else {
        Ok((input, lhs))
    }
}

fn parse_release(input: &str) -> IResult<&str, LTL> {
    let (input, lhs) = parse_until(input)?;

    if let Ok((next_input, rhs)) = preceded(ws(tag("R")), parse_release).parse(input) {
        Ok((next_input, LTL::Release(Box::new(lhs), Box::new(rhs))))
    } else {
        Ok((input, lhs))
    }
}

fn parse_weak_until(input: &str) -> IResult<&str, LTL> {
    let (input, lhs) = parse_release(input)?;

    if let Ok((next_input, rhs)) = preceded(ws(tag("W")), parse_weak_until).parse(input) {
        Ok((next_input, LTL::WeakUntil(Box::new(lhs), Box::new(rhs))))
    } else {
        Ok((input, lhs))
    }
}

fn parse_mighty_release(input: &str) -> IResult<&str, LTL> {
    let (input, lhs) = parse_weak_until(input)?;

    if let Ok((next_input, rhs)) = preceded(ws(tag("M")), parse_mighty_release).parse(input) {
        Ok((next_input, LTL::MightyRelease(Box::new(lhs), Box::new(rhs))))
    } else {
        Ok((input, lhs))
    }
}



fn parse_implies(input: &str) -> IResult<&str, LTL> {
    let (input, lhs) = parse_mighty_release(input)?;

    if let Ok((next_input, rhs)) = preceded(ws(tag("->")), parse_implies).parse(input) {
        Ok((next_input, LTL::Implies(Box::new(lhs), Box::new(rhs))))
    } else {
        Ok((input, lhs))
    }
}


fn parse_ltl(input: &str) -> IResult<&str, LTL> {
    parse_implies(input)
}


fn parse_property(input: &str) -> IResult<&str, (String, LTL)> {
    let (input, _) = ws(tag("Property")).parse(input)?;
    let (input, name) = ws(parse_identifier).parse(input)?;
    let (input, _) = ws(parse_quoted_string).parse(input)?;
    let (input, _) = ws(tag("is:")).parse(input)?;
    let (input, ltl) = parse_ltl(input)?;
    let (input, _) = ws(tag("end.")).parse(input)?;

    Ok((input, (name.to_string(), ltl)))
}


fn parse_mcc(input: &str) -> IResult<&str, Vec<(String, LTL)>> {
    let mut properties = Vec::new();

    let mut remaining = input;
    while let Ok((next_input, property)) = parse_property(remaining) {
        properties.push(property);
        remaining = next_input;
    }
    Ok((remaining, properties))
}

pub fn parse_mcc_file(path: &str) -> Result<Vec<(String, LTL)>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let (_, properties) = parse_mcc(&content).map_err(|e| format!("Parse error: {e:?}"))?;
    Ok(properties)
}





#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ltl_all(input: &str) -> LTL {
        let (remaining, parsed) = parse_ltl(input).expect("LTL parse should succeed");
        assert!(remaining.is_empty(), "unparsed suffix: {remaining:?}");
        parsed
    }

    fn parse_expr_all(input: &str) -> Expr {
        let (remaining, parsed) = parse_expr(input).expect("Expr parse should succeed");
        assert!(remaining.is_empty(), "unparsed suffix: {remaining:?}");
        parsed
    }

    #[test]
    fn test_parse_ltl() {
        let parsed = parse_ltl_all("G (a U b)");

        let expected = LTL::Globally(Box::new(LTL::Until(
            Box::new(LTL::Prop(Expr::Var("a".to_string()))),
            Box::new(LTL::Prop(Expr::Var("b".to_string()))),
        )));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_ltl_token() {
        let parsed = parse_ltl_all(
            "A X (X G X (((1) <= (#tokens(\"stp4\"))) & F (3 <= (#tokens(\"stp1\")))) & F X G ((2) <= (#tokens(\"AltitudePossibleVal\"))))",
        );

        let le_stp4 = LTL::Prop(Expr::LessEqual(
            Box::new(Expr::Number(1)),
            Box::new(Expr::TokenCount("stp4".to_string())),
        ));
        let le_stp1 = LTL::Prop(Expr::LessEqual(
            Box::new(Expr::Number(3)),
            Box::new(Expr::TokenCount("stp1".to_string())),
        ));
        let le_alt = LTL::Prop(Expr::LessEqual(
            Box::new(Expr::Number(2)),
            Box::new(Expr::TokenCount("AltitudePossibleVal".to_string())),
        ));

        let left_branch = LTL::Next(Box::new(LTL::Globally(Box::new(LTL::Next(Box::new(
            LTL::And(Box::new(le_stp4), Box::new(LTL::Eventually(Box::new(le_stp1)))),
        ))))));
        let right_branch = LTL::Eventually(Box::new(LTL::Next(Box::new(LTL::Globally(
            Box::new(le_alt),
        )))));

        let expected = LTL::AllPaths(Box::new(LTL::Next(Box::new(LTL::And(
            Box::new(left_branch),
            Box::new(right_branch),
        )))));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_ltl_firable() {
        let parsed = parse_ltl_all(
            "A X G X (\"t2_2\"? U X X !(X (\"t4_2\"? | \"SpeedRW\"?) U \"SpeedRW\"?))",
        );

        let fire_t22 = LTL::Prop(Expr::Fireable("t2_2".to_string()));
        let fire_t42_or_speed = LTL::Prop(Expr::Or(
            Box::new(Expr::Fireable("t4_2".to_string())),
            Box::new(Expr::Fireable("SpeedRW".to_string())),
        ));
        let fire_speed = LTL::Prop(Expr::Fireable("SpeedRW".to_string()));

        let nested_until = LTL::Until(
            Box::new(LTL::Next(Box::new(fire_t42_or_speed))),
            Box::new(fire_speed),
        );

        let expected = LTL::AllPaths(Box::new(LTL::Next(Box::new(LTL::Globally(Box::new(
            LTL::Next(Box::new(LTL::Until(
                Box::new(fire_t22),
                Box::new(LTL::Next(Box::new(LTL::Next(Box::new(LTL::Not(Box::new(
                    nested_until,
                ))))))),
            ))),
        ))))));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_expr_manual_tree() {
        let parsed = parse_expr_all("1 <= (#tokens(\"stp4\")) & !(\"t2_2\"?)");

        let expected = Expr::And(
            Box::new(Expr::LessEqual(
                Box::new(Expr::Number(1)),
                Box::new(Expr::TokenCount("stp4".to_string())),
            )),
            Box::new(Expr::Not(Box::new(Expr::Fireable("t2_2".to_string())))),
        );

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_ltl_manual_tree_until_and_unary() {
        let parsed = parse_ltl_all("A X (\"t2_2\"? U F (a & b))");

        let expected = LTL::AllPaths(Box::new(LTL::Next(Box::new(LTL::Until(
            Box::new(LTL::Prop(Expr::Fireable("t2_2".to_string()))),
            Box::new(LTL::Eventually(Box::new(LTL::Prop(Expr::And(
                Box::new(Expr::Var("a".to_string())),
                Box::new(Expr::Var("b".to_string())),
            ))))),
        )))));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_expr_implies_is_right_associative() {
        let parsed = parse_expr_all("a -> b -> c");

        let expected = Expr::Implies(
            Box::new(Expr::Var("a".to_string())),
            Box::new(Expr::Implies(
                Box::new(Expr::Var("b".to_string())),
                Box::new(Expr::Var("c".to_string())),
            )),
        );

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_mcc_file() {
        let properties = parse_mcc_file("pnml/test_files/test.txt");
        match properties {
            Ok(props) => {
                assert_eq!(props.len(), 3);
            }
            Err(e) => panic!("Failed to parse MCC file: {e}"),
        }
    }
}