use logos::Logos;
use parse_it::ParseIt;

#[derive(Logos, Clone, Eq, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token {
    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Mul,

    #[token("/")]
    Div,

    #[token(">")]
    Right,

    #[token("<")]
    Left,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[regex(r"\d+", |lex| lex.slice().parse::<i32>().unwrap())]
    Number(i32),
}

impl PartialEq<char> for Token {
    fn eq(&self, other: &char) -> bool {
        match dbg!(self) {
            Token::Plus => *other == '+',
            Token::Minus => *other == '-',
            Token::Mul => *other == '*',
            Token::Div => *other == '/',
            Token::Right => *other == '>',
            Token::Left => *other == '<',
            Token::LParen => *other == '(',
            Token::RParen => *other == ')',
            _ => false,
        }
    }
}

parse_it::parse_it! {
    #[parse_it(debug = true)]
    mod parse {
        use super::Token;

        type Lexer<'a> = parse_it::LogosLexer<'a, Token>;

        pub Expr -> i32 {
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
            Token::Number(num) => num,
            '(' expr:Expr ')' => expr,
        }
    }

}

fn main() {
    let input = "1 + 2 * (3 - 4)";
    let parser = parse::Expr::default();
    let result = parser.parse(input).unwrap();
    println!("{:?}", result);
}
