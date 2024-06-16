#![allow(clippy::just_underscores_and_digits, clippy::redundant_pattern)]
use chumsky::{primitive::end, Parser as _};
use parse_it::Parser;

fn parser0<'src>() -> Parser<'src, i32> {
    parse_it::parse_it! {
        Digit -> char {
            @['0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9'] => self
        }

        Num -> i32 {
            digits:Digit+ => digits.into_iter().collect::<String>().parse::<i32>().unwrap(),
        }

        Expr -> i32 {
            AddExpr => self,
        }

        AddExpr -> i32 {
            lhs:MulExpr '+' rhs:AddExpr => {
                lhs + rhs
            },
            lhs:MulExpr '-' rhs:AddExpr => {
                lhs - rhs
            },
            MulExpr => self,
        }

        MulExpr -> i32 {
            lhs:Term '*' rhs:MulExpr => {
                lhs * rhs
            },
            lhs:Term '/' rhs:MulExpr => {
                lhs / rhs
            },
            Term => self,
        }

        Term -> i32 {
            Num => self,
            '(' expr:Expr ')' => expr,
        }

        return Expr;
    }
}

fn parser<'src>() -> Parser<'src, i32> {
    use chumsky::prelude::*;

    let mut expr = Recursive::<_, _, Simple<char>>::declare();
    let mut add_expr = Recursive::declare();
    let mut mul_expr = Recursive::declare();

    let digit = choice("0123456789".chars().map(just).collect::<Vec<_>>());

    let num = digit
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|digits| digits.parse::<i32>().unwrap());

    let term = choice((
        num.clone(),
        just('(')
            .ignore_then(expr.clone().padded())
            .then_ignore(just(')')),
    ));

    expr.define(add_expr.clone());

    add_expr.define(choice((
        mul_expr
            .clone()
            .padded()
            .then_ignore(just('+'))
            .then(add_expr.clone().padded())
            .map(|(lhs, rhs)| lhs + rhs),
        mul_expr
            .clone()
            .padded()
            .then_ignore(just('-'))
            .then(add_expr.clone().padded())
            .map(|(lhs, rhs)| lhs - rhs),
        mul_expr.clone(),
    )));

    mul_expr.define(choice((
        term.clone()
            .padded()
            .then_ignore(just('*'))
            .then(mul_expr.clone().padded())
            .map(|(lhs, rhs)| lhs * rhs),
        term.clone()
            .padded()
            .then_ignore(just('/'))
            .then(mul_expr.clone().padded())
            .map(|(lhs, rhs)| lhs / rhs),
        term.clone(),
    )));

    expr.boxed()
}

fn main() {
    let input = "11+2*(3+4)/5";

    let parser = parser().then_ignore(end());
    let result = parser.parse(input).unwrap();
    println!("parser: {}", result);
    assert_eq!(result, 13);

    let parser = parser0().then_ignore(end());
    let result = parser.parse(input).unwrap();
    println!("parser0: {}", result);
    assert_eq!(result, 13);
}
