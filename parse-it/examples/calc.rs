use parse_it::ParseIt;

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
        lhs:AddExpr '+' rhs:MulExpr => {
            lhs + rhs
        }
        lhs:AddExpr '-' rhs:MulExpr => {
            lhs - rhs
        }
        MulExpr => self,
    }

    MulExpr -> i32 {
        lhs:MulExpr '*' rhs:Term => {
            lhs * rhs
        }
        lhs:MulExpr '/' rhs:Term => {
            lhs / rhs
        }
        Term => self,
    }

    Term -> i32 {
        Num => self,
        '(' expr:Expr ')' => expr,
    }

    return Expr;
}

fn main() {
    let parser = Expr::default();

    let input = "11+(6-1-1)*(4/2/2)";

    let result = match parser.parse(input) {
        Ok(value) => value,
        Err(err) => {
            println!("span: {}..{}", err.span.0, err.span.1);
            println!("{}", err.backtrace);
            return;
        }
    };

    println!("parser: {}", result);
    assert_eq!(result, 15);
}
