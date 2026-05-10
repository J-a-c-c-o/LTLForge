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
pub enum LTL {
    True,
    False,
    Var(String),
    Not(Box<LTL>),
    And(Box<LTL>, Box<LTL>),
    Or(Box<LTL>, Box<LTL>),
    Implies(Box<LTL>, Box<LTL>),
    LessEqual(Box<LTL>, Box<LTL>),
    GreaterEqual(Box<LTL>, Box<LTL>),
    Greater(Box<LTL>, Box<LTL>),
    Less(Box<LTL>, Box<LTL>),
    TokenCount(String),
    Fireable(String),
    Number(u32),
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

impl std::fmt::Display for LTL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LTL::True => write!(f, "true"),
            LTL::False => write!(f, "false"),
            LTL::Var(name) => write!(f, "{}", name),
            LTL::Not(inner) => write!(f, "!({})", inner),
            LTL::And(left, right) => write!(f, "({} & {})", left, right),
            LTL::Or(left, right) => write!(f, "({} | {})", left, right),
            LTL::Implies(left, right) => write!(f, "({} -> {})", left, right),
            LTL::LessEqual(left, right) => write!(f, "({} <= {})", left, right),
            LTL::GreaterEqual(left, right) => write!(f, "({} >= {})", left, right),
            LTL::Greater(left, right) => write!(f, "({} > {})", left, right),
            LTL::Less(left, right) => write!(f, "({} < {})", left, right),
            LTL::TokenCount(name) => write!(f, "#tokens(\"{}\")", name),
            LTL::Fireable(name) => write!(f, "\"{}\"?", name),
            LTL::Number(n) => write!(f, "{}", n),
            LTL::Next(inner) => write!(f, "X {}", inner),
            LTL::Eventually(inner) => write!(f, "F {}", inner),
            LTL::Globally(inner) => write!(f, "G {}", inner),
            LTL::AllPaths(inner) => write!(f, "A {}", inner),
            LTL::SomePath(inner) => write!(f, "E {}", inner),
            LTL::Until(left, right) => write!(f, "({} U {})", left, right),
            LTL::WeakUntil(left, right) => write!(f, "({} W {})", left, right),
            LTL::Release(left, right) => write!(f, "({} R {})", left, right),
            LTL::MightyRelease(left, right) => write!(f, "({} M {})", left, right),
        }
    }
}

impl LTL {
    pub fn size(&self) -> usize {
        match self {
            LTL::True | LTL::False | LTL::Var(_) | LTL::TokenCount(_) | LTL::Fireable(_) | LTL::Number(_) => 1,
            LTL::Not(inner) => 1 + inner.size(),
            LTL::Next(inner) | LTL::Eventually(inner) | LTL::Globally(inner) | LTL::AllPaths(inner) | LTL::SomePath(inner) => 1 + inner.size(),
            LTL::And(left, right)
            | LTL::Or(left, right)
            | LTL::Implies(left, right)
            | LTL::LessEqual(left, right)
            | LTL::GreaterEqual(left, right)
            | LTL::Greater(left, right)
            | LTL::Less(left, right)
            | LTL::Until(left, right)
            | LTL::WeakUntil(left, right)
            | LTL::Release(left, right)
            | LTL::MightyRelease(left, right) => 1 + left.size() + right.size(),
        }
    }


    pub fn negate(&self) -> LTL {
        LTL::Not(Box::new(self.clone()))
    }
}

fn parse_expr(input: &str) -> IResult<&str, LTL> {
    parse_expr_implies(input)
}

fn parse_expr_implies(input: &str) -> IResult<&str, LTL> {
    let (input, lhs) = parse_expr_or(input)?;

    if let Ok((next_input, rhs)) = preceded(ws(tag("->")), parse_expr_implies).parse(input) {
        Ok((next_input, LTL::Implies(Box::new(lhs), Box::new(rhs))))
    } else {
        Ok((input, lhs))
    }
}

fn parse_expr_or(input: &str) -> IResult<&str, LTL> {
    let (mut input, mut expr) = parse_expr_and(input)?;

    while let Ok((next_input, rhs)) = preceded(ws(alt((tag("||"), tag("|")))), parse_expr_and)
        .parse(input)
    {
        expr = LTL::Or(Box::new(expr), Box::new(rhs));
        input = next_input;
    }

    Ok((input, expr))
}

fn parse_expr_and(input: &str) -> IResult<&str, LTL> {
    let (mut input, mut expr) = parse_expr_compare(input)?;

    while let Ok((next_input, rhs)) = preceded(ws(alt((tag("&&"), tag("&")))), parse_expr_compare)
        .parse(input)
    {
        expr = LTL::And(Box::new(expr), Box::new(rhs));
        input = next_input;
    }

    Ok((input, expr))
}

fn parse_expr_compare(input: &str) -> IResult<&str, LTL> {
    let (input, lhs) = parse_expr_not(input)?;

    if let Ok((next_input, _)) = parse_le_operator(input) {
        let (next_input, rhs) = parse_expr_not(next_input)?;
        Ok((next_input, LTL::LessEqual(Box::new(lhs), Box::new(rhs))))
    } else if let Ok((next_input, _)) = parse_ge_operator(input) {
        let (next_input, rhs) = parse_expr_not(next_input)?;
        Ok((next_input, LTL::GreaterEqual(Box::new(lhs), Box::new(rhs))))
    } else if let Ok((next_input, _)) = parse_gt_operator(input) {
        let (next_input, rhs) = parse_expr_not(next_input)?;
        Ok((next_input, LTL::Greater(Box::new(lhs), Box::new(rhs))))
    } else if let Ok((next_input, _)) = parse_lt_operator(input) {
        let (next_input, rhs) = parse_expr_not(next_input)?;
        Ok((next_input, LTL::Less(Box::new(lhs), Box::new(rhs))))
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

fn parse_gt_operator(input: &str) -> IResult<&str, &str> {
    ws(tag(">")).parse(input)
}

fn parse_lt_operator(input: &str) -> IResult<&str, &str> {
    ws(tag("<")).parse(input)
}

fn parse_expr_not(input: &str) -> IResult<&str, LTL> {
    alt((
        map(preceded(ws(char('!')), parse_expr_not), |expr| {
            LTL::Not(Box::new(expr))
        }),
        parse_expr_primary,
    ))
    .parse(input)
}

fn parse_expr_primary(input: &str) -> IResult<&str, LTL> {
    alt((
        value(LTL::True, ws(tag("true"))),
        value(LTL::False, ws(tag("false"))),
        parse_token_count,
        parse_fireable,
        map(ws(digit1), |n: &str| LTL::Number(n.parse().unwrap())),
        parse_variable_expr,
        delimited(ws(char('(')), parse_expr, ws(char(')'))),
    ))
    .parse(input)
}

fn parse_variable_expr(input: &str) -> IResult<&str, LTL> {
    let (input, name) = ws(parse_identifier).parse(input)?;

    if matches!(name, "A" | "E" | "X" | "F" | "G" | "U" | "R" | "W" | "M") {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    Ok((input, LTL::Var(name.to_string())))
}

fn parse_token_count(input: &str) -> IResult<&str, LTL> {
    map(
        preceded(
            ws(tag("#tokens")),
            delimited(ws(char('(')), ws(parse_quoted_string), ws(char(')'))),
        ),
        |name| LTL::TokenCount(name.to_string()),
    )
    .parse(input)
}

fn parse_fireable(input: &str) -> IResult<&str, LTL> {
    map(terminated(ws(parse_quoted_string), ws(char('?'))), |name| {
        LTL::Fireable(name.to_string())
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

fn ws<'a, O, E, P>(parser: P) -> impl Parser<&'a str, Output = O, Error = E>
where
    E: nom::error::ParseError<&'a str>,
    P: Parser<&'a str, Output = O, Error = E>,
{
    delimited(multispace0, parser, multispace0)
}

fn parse_primary(input: &str) -> IResult<&str, LTL> {
    alt((
        delimited(ws(char('(')), parse_ltl, ws(char(')'))),
        parse_expr_primary,
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

pub(crate) fn parse_ltl(input: &str) -> IResult<&str, LTL> {
    let (input, ltl) = parse_implies(input)?;

    let initial_ltl = match ltl {
        LTL::AllPaths(inner) => *inner,
        other => other,
    };

    let mut stack = vec![&initial_ltl];
    while let Some(current) = stack.pop() {
        match current {
            LTL::AllPaths(_) | LTL::SomePath(_) => {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Tag,
                )));
            }
            LTL::Not(inner) => stack.push(inner),
            LTL::And(left, right) | LTL::Or(left, right) | LTL::Implies(left, right) => {
                stack.push(left);
                stack.push(right);
            }
            LTL::Until(left, right)
            | LTL::WeakUntil(left, right)
            | LTL::Release(left, right)
            | LTL::MightyRelease(left, right) => {
                stack.push(left);
                stack.push(right);
            }
            LTL::Next(inner) | LTL::Eventually(inner) | LTL::Globally(inner) => stack.push(inner),
            _ => {}
        }
    }

    Ok((input, initial_ltl))
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

    fn parse_ltl_all(input: &str) -> Result<LTL, nom::Err<nom::error::Error<&str>>> {
        let (remaining, parsed) = parse_ltl(input)?;
        assert!(remaining.is_empty(), "unparsed suffix: {remaining:?}");
        Ok(parsed)
    }

    fn parse_expr_all(input: &str) -> Result<LTL, nom::Err<nom::error::Error<&str>>> {
        let (remaining, parsed) = parse_expr(input)?;
        assert!(remaining.is_empty(), "unparsed suffix: {remaining:?}");
        Ok(parsed)
    }

    #[test]
    fn test_parse_ltl() {
        let parsed = parse_ltl_all("G (a U b)").unwrap();

        let expected = LTL::Globally(Box::new(LTL::Until(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::Var("b".to_string())),
        )));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_neg_and_in_until() {
        let parsed = parse_ltl_all("G (a U (!a & b))").unwrap();

        let expected = LTL::Globally(Box::new(LTL::Until(
            Box::new(LTL::Var("a".to_string())),
            Box::new(LTL::And(
                Box::new(LTL::Not(Box::new(LTL::Var("a".to_string())))),
                Box::new(LTL::Var("b".to_string())),
            )),
        )));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_expr_manual_tree() {
        let parsed = parse_expr_all("1 <= (#tokens(\"stp4\")) & !(\"t2_2\"?)").unwrap();

        let expected = LTL::And(
            Box::new(LTL::LessEqual(
                Box::new(LTL::Number(1)),
                Box::new(LTL::TokenCount("stp4".to_string())),
            )),
            Box::new(LTL::Not(Box::new(LTL::Fireable("t2_2".to_string())))),
        );

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_ltl_firable() {
        let parsed = parse_ltl_all(
            "A X G X (\"t2_2\"? U X X !(X (\"t4_2\"? | \"SpeedRW\"?) U \"SpeedRW\"?))",
        )
        .unwrap();

        let fire_t22 = LTL::Fireable("t2_2".to_string());
        let fire_t42_or_speed = LTL::Or(
            Box::new(LTL::Fireable("t4_2".to_string())),
            Box::new(LTL::Fireable("SpeedRW".to_string())),
        );
        let fire_speed = LTL::Fireable("SpeedRW".to_string());

        let nested_until = LTL::Until(
            Box::new(LTL::Next(Box::new(fire_t42_or_speed))),
            Box::new(fire_speed),
        );

        let expected = LTL::Next(Box::new(LTL::Globally(Box::new(
            LTL::Next(Box::new(LTL::Until(
                Box::new(fire_t22),
                Box::new(LTL::Next(Box::new(LTL::Next(Box::new(LTL::Not(Box::new(
                    nested_until,
                ))))))),
            ))),
        ))));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_invalid() {
        assert!(parse_ltl_all("E X (a U b)").is_err());
    }
}
