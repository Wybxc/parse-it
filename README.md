# Parse It

*A user-friendly, opinionated parser generator for Rust.*

## Example

```rust
use parse_it::{ParseIt, parse_it};

type Lexer = parse_it::CharLexer;

#[derive(Debug, Clone)]
pub enum Instr {
    Left,
    Right,
    Incr,
    Decr,
    Read,
    Write,
    Loop(Vec<Self>),
}

parse_it! {
    #[parser]
    mod parse {
        use super::Instr;

        pub Brainfuck -> Vec<Instr> {
            Primitive* => self,
        }

        Primitive -> Instr {
            '<' => Instr::Left,
            '>' => Instr::Right,
            '+' => Instr::Incr,
            '-' => Instr::Decr,
            ',' => Instr::Read,
            '.' => Instr::Write,
            '[' Primitive+ ']' => Instr::Loop(self)
        }
    }
}

fn main() {
    let parser = parse::Brainfuck::default();
    let src = "--[>--->->->++>-<<<<<-------]>--.>---------.>--..+++.>----.>+++++++++.<<.+++.------.<-.>>+";
    let instrs = parser.parse(src).unwrap();
    println!("{:?}", instrs);
}
```

## Planned features

- [x] Parser generation
- [x] Lexer generation
- [ ] Error reporting
- [ ] Error recovery
- [ ] Grammar lints
