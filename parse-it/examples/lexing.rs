use parse_it::{LexIt, LexerState};

parse_it::parse_it! {
    #[lexer]
    mod lex {
        use parse_it::lexer::Token;

        pub Initial -> Token<'lex, ()> {
            r"\s" => continue,
            "\"" => {
                let mut buf = String::new();
                while lex!(StringLiteral(&mut buf)).is_some() {}
                buf.into()
            },
            Integer => self.into(),
            r"[\p{XID_Start}_]\p{XID_Continue}*" => self.into(),
        }

        Integer -> i64 {
            r"\d+" => self.parse::<i64>().unwrap(),
        }

        StringLiteral(buf: &mut String) {
            r#"""# => break,
            r"\\n" => buf.push('\n'),
            r#"\\""# => buf.push('"'),
            r"\\." => buf.push(self.chars().nth(1).unwrap()),
            r"." => buf.push_str(self),
        }
    }
}

fn main() {
    let src = r#"
        "Hello, World!"
        42
        identifier
    "#;
    let lexer = lex::Initial::new();
    let mut lexbuf = LexerState::new(src);
    while let Some(token) = lexer.next(&mut lexbuf) {
        println!("{token:?}");
    }
}
