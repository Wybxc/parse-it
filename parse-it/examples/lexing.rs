parse_it::parse_it! {
    #[lexer]
    mod lex {
        pub Initial -> Token {
            r"\s" => continue,
            "\"" => {
                let mut buf = String::new();
                while lex!(StringLiteral(&mut buf)).is_some() {}
                buf
            },
            Integer => self,
            r"[\p{XID_Start}_]\p{XID_Continue}*" => self,
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

#[allow(
    dead_code,
    unreachable_code,
    clippy::never_loop,
    clippy::let_unit_value,
    clippy::unit_arg,
    clippy::useless_conversion
)]
mod lex_ {
    use parse_it::lexer::{LexerState, Token};
    use regex_automata::meta::Regex;

    pub struct Initial;

    impl Initial {
        thread_local! {
            static REGEX: Regex = Regex::new_many(&[
                r"\s",
                r#"""#,
                r"\d+",
                r"[\p{XID_Start}_]\p{XID_Continue}*",
            ]).unwrap();
        }

        pub fn run(lexbuf: &mut LexerState) -> Result<Option<Token<()>>, ()> {
            Self::REGEX.with(|regex| {
                'lex: loop {
                    if let Some(pat) = lexbuf.run(regex) {
                        let __self = lexbuf.lexeme();
                        let value = match pat.as_i32() {
                            0 => continue 'lex,
                            1 => {
                                let mut buf = String::new();
                                while StringLiteral::run(lexbuf, &mut buf)?.is_some() {}
                                buf
                            }
                            .into(),
                            2 => {
                                let __self: i64 = __self.parse::<i64>().unwrap().into();
                                __self.into()
                            }
                            3 => __self.into(),
                            _ => unreachable!(),
                        };
                        return Ok(Some(value));
                    } else {
                        return Err(());
                    }
                }
                Ok(None)
            })
        }
    }

    struct StringLiteral;

    impl StringLiteral {
        thread_local! {
            static REGEX: Regex = Regex::new_many(&[
                r#"""#,
                r"\\n",
                r#"\\""#,
                r"\\.",
                r".",
            ]).unwrap();
        }

        pub fn run(lexbuf: &mut LexerState, buf: &mut String) -> Result<Option<()>, ()> {
            Self::REGEX.with(|regex| {
                'lex: loop {
                    if let Some(pat) = lexbuf.run(regex) {
                        let __self = lexbuf.lexeme();
                        let value = match pat.as_i32() {
                            0 => break 'lex,
                            1 => buf.push('\n').into(),
                            2 => buf.push('"').into(),
                            3 => buf.push(__self.chars().nth(1).unwrap()).into(),
                            4 => buf.push_str(__self).into(),
                            _ => unreachable!(),
                        };
                        return Ok(Some(value));
                    } else {
                        return Err(());
                    }
                }
                Ok(None)
            })
        }
    }
}

fn main() {}
