#![feature(test)]

extern crate test;

use rune::{Hash, Result, Vm};
use rune_tests::modules::capture_io::CaptureIo;
use test::Bencher;

fn make_vm() -> Result<(Vm, CaptureIo)> {
    Ok(rune_tests::rune_vm_capture! {
    enum Op {
        Inc(v),
        Move(v),
        Loop(ops),
        Input,
        Print,
    }

    struct Tape {
        pos,
        tape,
    }

    impl Tape {
        fn new() {
            Tape { pos: 0, tape: [0] }
        }
        fn get(self) { self.tape[self.pos] }
        fn getc(self)  { std::char::from_int(self.get()).expect("a valid char")  }
        fn inc(self, x) {
            self.tape[self.pos] = (self.tape[self.pos] + x) % 256;
            if self.tape[self.pos] < 0 {
                self.tape[self.pos] = self.tape[self.pos] + 256;
            }
        }
        fn mov(self, x) {
            self.pos = (self.pos + x);
            while self.pos >= self.tape.len() { self.tape.push(0); }
        }

        fn set(self, v) {
            self.tape[self.pos] = v;
        }
    }

    fn run(program, tape, inputs) {
        for op in program {
            match op {
                Op::Inc(x) => tape.inc(x),
                Op::Move(x) => tape.mov(x),
                Op::Loop(program) => while tape.get() != 0 {
                    run(program, tape, inputs);
                },
                Op::Print => {
                    let c = tape.getc();
                    print(format!("{}", c));
                }
                Op::Input => {
                    tape.set(0)
                }
            }
        }
    }

    fn parse(it) {
        let buf = Vec::new();

        while let Some(c) = it.next() {
            match c {
                '+' => buf.push(Op::Inc(1)),
                '-' => buf.push(Op::Inc(-1)),
                '>' => buf.push(Op::Move(1)),
                '<' => buf.push(Op::Move(-1)),
                '.' => buf.push(Op::Print),
                '[' => buf.push(Op::Loop(parse(it))),
                ',' => buf.push(Op::Input),
                ']' => break,
                _ => continue,
            };
        }
        buf
    }

    struct Program {
        ops,
        inputs
    }

    impl Program {
        fn new(code, inputs) { Program { ops: parse(code), inputs } }
        fn run(self) {
            let tape = Tape::new();
            run(self.ops, tape, self.inputs);
        }
    }

    pub fn main(s, i) {
        let program = Program::new(s.chars(), i);
        program.run();

    }
    })
}

#[bench]
fn brainfuck_hello_world(b: &mut Bencher) -> Result<()> {
    let (mut vm, io) = make_vm()?;

    let program = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";

    let entry = Hash::type_hash(&["main"]);

    b.iter(|| {
        let value = vm.call(entry, (program, 0)).expect("successful execution");
        let out = io.drain_utf8();
        assert_eq!(out.as_deref(), Ok("Hello World!\n"));
        value
    });

    Ok(())
}

#[bench]
fn brainfuck_hello_world2(b: &mut Bencher) -> Result<()> {
    let (mut vm, io) = make_vm()?;

    // interesting hello world which wraps cells on the negative side
    let program = "+[-[<<[+[--->]-[<<<]]]>>>-]>-.---.>..>.<<<<-.<+.>>>>>.>.<<.<-.";

    let entry = Hash::type_hash(&["main"]);

    b.iter(|| {
        let value = vm.call(entry, (program, 0)).expect("successful execution");
        let out = io.drain_utf8();
        assert_eq!(out.as_deref(), Ok("hello world"));
        value
    });

    Ok(())
}

#[bench]
fn brainfuck_fib(b: &mut Bencher) -> Result<()> {
    let (mut vm, io) = make_vm()?;

    // Computes the first 16 fib numbers
    let program = "++++++++++++++++++++++++++++++++++++++++++++>++++++++++++++++++++++++++++++++>++ ++++++++++++++>>+<<[>>>>++++++++++<<[->+>-[>+>>]>[+[-<+>]>+>>]<<<<<<]>[<+>-]>[-] >>>++++++++++<[->-[>+>>]>[+[-<+>]>+>>]<<<<<]>[-]>>[+++++++++++++++++++++++++++++ +++++++++++++++++++.[-]]<[++++++++++++++++++++++++++++++++++++++++++++++++.[-]]< <<++++++++++++++++++++++++++++++++++++++++++++++++.[-]<<<<<<<.>.>>[>>+<<-]>[>+<< +>-]>[<+>-]<<<-]<<++...";
    let entry = Hash::type_hash(&["main"]);

    b.iter(|| {
        let value = vm.call(entry, (program, 0)).expect("successful execution");
        let out = io.drain_utf8();
        assert_eq!(
            out.as_deref(),
            Ok("1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 121, 98, 219, ...")
        );
        value
    });

    Ok(())
}

#[bench]
fn brainfuck_loopity(b: &mut Bencher) -> Result<()> {
    let (mut vm, io) = make_vm()?;

    // Just a program that runs a lot of instructions
    let program = ">+[>++>+++[-<]>>]+";

    let entry = Hash::type_hash(&["main"]);

    b.iter(|| {
        let value = vm.call(entry, (program, 5)).expect("successful execution");
        let out = io.drain_utf8();
        assert_eq!(out.as_deref(), Ok(""));
        value
    });

    Ok(())
}
