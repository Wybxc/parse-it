use std::io::{BufRead, BufReader, Bytes, Read};

use parse_it::ParseIt;

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

parse_it::parse_it! {
    #[parser]
    mod parse {
        use super::Instr;

        type Lexer = parse_it::CharLexer;

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
    let src = "--[>--->->->++>-<<<<<-------]>--.>---------.>--..+++.>----.>+++++++++.<<.+++.------.<-.>>+.";

    match parser.parse(src) {
        Ok(ast) => {
            let mut stdin = BufReader::new(std::io::stdin().lock()).bytes();
            execute(&ast, &mut 0, &mut [0; TAPE_LEN], &mut stdin)
        }
        Err(err) => println!("{err:?}"),
    };
}

const TAPE_LEN: usize = 10_000;

fn execute(
    ast: &[Instr],
    ptr: &mut usize,
    tape: &mut [u8; TAPE_LEN],
    stdin: &mut Bytes<impl BufRead>,
) {
    for symbol in ast {
        match symbol {
            Instr::Left => *ptr = (*ptr + TAPE_LEN - 1).rem_euclid(TAPE_LEN),
            Instr::Right => *ptr = (*ptr + 1).rem_euclid(TAPE_LEN),
            Instr::Incr => tape[*ptr] = tape[*ptr].wrapping_add(1),
            Instr::Decr => tape[*ptr] = tape[*ptr].wrapping_sub(1),
            Instr::Read => tape[*ptr] = stdin.next().unwrap().unwrap(),
            Instr::Write => print!("{}", tape[*ptr] as char),
            Instr::Loop(ast) => {
                while tape[*ptr] != 0 {
                    execute(ast, ptr, tape, stdin)
                }
            }
        }
    }
}
