# Parse It

*A user-friendly, opinionated parser generator for Rust.*

## Example

```rust
#[derive(Debug, Clone)]
enum Instr {
    Left,
    Right,
    Incr,
    Decr,
    Read,
    Write,
    Loop(Vec<Self>),
}

let parser = parse_it::parse_it! {
    Brainfuck -> Vec<Instr> {
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

    return Brainfuck;
};

let src = "--[>--->->->++>-<<<<<-------]>--.>---------.>--..+++.>----.>+++++++++.<<.+++.------.<-.>>+.";

let instrs = parser.parse(src).unwrap();
println!("{:?}", instrs);
```
