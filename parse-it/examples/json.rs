use std::collections::HashMap;

use parse_it::ParseIt;

#[derive(Clone, Debug)]
pub enum JsonValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

parse_it::parse_it! {
    #[lexer]
    mod lex {
        #[derive(Debug)]
        pub enum Token {
            Number(f64),
            String(String),
            Keyword,
        }

        pub Initial -> Token {
            r"\s+" => continue, // Skip whitespace
            Number => Token::Number(self),
            "\"" => {
                let mut buf = String::new();
                while lex!(StringLit(&mut buf)).is_some() {}
                Token::String(buf)
            },
            "true" => Token::Keyword,
            "false" => Token::Keyword,
            "null" => Token::Keyword,
            r"\[" => Token::Keyword,
            r"\]" => Token::Keyword,
            r"\{" => Token::Keyword,
            r"\}" => Token::Keyword,
            r"," => Token::Keyword,
            r":" => Token::Keyword,
        }

        Number -> f64 {
            r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?" => self.parse().unwrap(),
        }

        StringLit(buf: &mut String) {
            "\"" => break,
            r"\\n" => buf.push('\n'),
            r"\\t" => buf.push('\t'),
            r#"\\\""# => buf.push('"'),
            r"\\\\" => buf.push('\\'),
            r"\\/" => buf.push('/'),
            r"\\b" => buf.push('\x08'),
            r"\\f" => buf.push('\x0C'),
            r"\\r" => buf.push('\r'),
            r#"[^\"\\]"# => buf.push_str(self),
        }
    }

    #[parser]
    mod parse {
        use std::collections::HashMap;
        use super::JsonValue;
        use super::lex::Token;

        type Lexer = super::Debug;

        Object -> JsonValue {
            '{' '}' => JsonValue::Object(HashMap::new()),
            '{' ps:( Key ':' Value ',' )* p:( Key ':' Value ) '}' => {
                let map = ps.into_iter().chain(std::iter::once(p)).collect::<HashMap<_, _>>();
                JsonValue::Object(map)
            }
        }

        Array -> JsonValue {
            '[' ']' => JsonValue::Array(Vec::new()),
            '[' vs:(Value ',')* v:Value ']' => {
                let vec = vs.into_iter().chain(std::iter::once(v)).collect();
                JsonValue::Array(vec)
            }
        }

        Key -> String {
            Token::String(buf) => buf.clone()
        }

        pub Value -> JsonValue {
            Token::Number(i) => JsonValue::Number(i),
            Token::String(buf) => JsonValue::String(buf.clone()),
            "true" => JsonValue::Boolean(true),
            "false" => JsonValue::Boolean(false),
            "null" => JsonValue::Null,
            Object => self,
            Array => self,
        }
    }
}

#[derive(Clone)]
pub struct Debug;

impl parse_it::LexIt for Debug {
    type Token<'a> = lex::Token;

    fn new() -> Self {
        Self
    }

    fn next<'a>(&self, lexbuf: &mut parse_it::LexerState<'a>) -> Option<Self::Token<'a>> {
        let result = lex::Initial.next(lexbuf);
        eprintln!("Lexing: {:?} at {:?}", result, lexbuf.span());
        result
    }
}

fn main() {
    let input = r#"{"name": "Alice", "age": 30, "is_student": false, "courses": ["Math", "Science"], "address": null}"#;

    let parser = parse::Value::default();
    let json = parser.parse(input).unwrap();
    println!("{json:#?}");
}
