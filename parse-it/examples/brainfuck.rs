use std::io::Read;

#[derive(Clone)]
enum Instr {
    Left,
    Right,
    Incr,
    Decr,
    Read,
    Write,
    Loop(Vec<Self>),
}

fn main() {
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

    match parser.parse(src) {
        Ok(ast) => execute(&ast, &mut 0, &mut [0; TAPE_LEN]),
        Err(errs) => errs.into_iter().for_each(|e| println!("{:?}", e)),
    };
}

const TAPE_LEN: usize = 10_000;

fn execute(ast: &[Instr], ptr: &mut usize, tape: &mut [u8; TAPE_LEN]) {
    for symbol in ast {
        match symbol {
            Instr::Left => *ptr = (*ptr + TAPE_LEN - 1).rem_euclid(TAPE_LEN),
            Instr::Right => *ptr = (*ptr + 1).rem_euclid(TAPE_LEN),
            Instr::Incr => tape[*ptr] = tape[*ptr].wrapping_add(1),
            Instr::Decr => tape[*ptr] = tape[*ptr].wrapping_sub(1),
            Instr::Read => tape[*ptr] = std::io::stdin().bytes().next().unwrap().unwrap(),
            Instr::Write => print!("{}", tape[*ptr] as char),
            Instr::Loop(ast) => {
                while tape[*ptr] != 0 {
                    execute(ast, ptr, tape)
                }
            }
        }
    }
}
