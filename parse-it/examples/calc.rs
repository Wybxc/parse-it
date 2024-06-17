use parse_it::Parser;

fn parser() -> Parser<i32> {
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
            }
            lhs:MulExpr '-' rhs:AddExpr => {
                lhs - rhs
            }
            MulExpr => self,
        }

        MulExpr -> i32 {
            lhs:Term '*' rhs:MulExpr => {
                lhs * rhs
            }
            lhs:Term '/' rhs:MulExpr => {
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
}

fn main() {
    let input = "11+2*(3+4)/5";

    let parser = parser();
    let result = parser.parse(input).unwrap();
    println!("parser: {}", result);
    assert_eq!(result, 13);
}
