use parse_it::ParseIt;

parse_it::parse_it! {
    #[lexer]
    mod lex {
        pub Initial {
            r"\s" => (),
            Integer => self,
            r"[\p{XID_Start}_]\p{XID_Continue}*" => self,
        }

        Integer -> i64 {
            r"\d+" => self.parse::<i64>().unwrap(),
        }
    }
}

fn main() {}
