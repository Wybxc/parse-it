parse_it::parse_it! {
    #[lexer]
    mod lex {
        use parse_it::lexer::Token;

        pub Initial -> Token<'lex, ()> {
            Integer => self.into(),
        }

        Integer -> f64 {
            r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?" => self.parse().unwrap(),
        }
    }
}

fn main() {}
