use criterion::Criterion;
use rune::support::Result;
use rune::{Hash, Vm};

use rune::modules::capture_io::CaptureIo;

criterion::criterion_group!(benches, entry);

fn entry(b: &mut Criterion) {
    let (mut vm, io) = make_vm().unwrap();

    b.bench_function("brainfuck_hello_world", move |b| {
        let program = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
        let entry = Hash::type_hash(["main"]);

        b.iter(|| {
            let value = vm.call(entry, (program, 0)).expect("failed call");
            let out = io.drain_utf8();
            assert_eq!(out.as_deref(), Ok("Hello World!\n"));
            value
        });
    });

    let (mut vm, io) = make_vm().unwrap();

    b.bench_function("brainfuck_hello_world2", |b| {
        // interesting hello world which wraps cells on the negative side
        let program = "+[-[<<[+[--->]-[<<<]]]>>>-]>-.---.>..>.<<<<-.<+.>>>>>.>.<<.<-.";
        let entry = Hash::type_hash(["main"]);

        b.iter(|| {
            let value = vm.call(entry, (program, 0)).expect("failed call");
            let out = io.drain_utf8();
            assert_eq!(out.as_deref(), Ok("hello world"));
            value
        });
    });

    let (mut vm, io) = make_vm().unwrap();

    b.bench_function("brainfuck_fib", |b| {
        // Computes the first 16 fib numbers
        let program = "++++++++++++++++++++++++++++++++++++++++++++>++++++++++++++++++++++++++++++++>++ ++++++++++++++>>+<<[>>>>++++++++++<<[->+>-[>+>>]>[+[-<+>]>+>>]<<<<<<]>[<+>-]>[-] >>>++++++++++<[->-[>+>>]>[+[-<+>]>+>>]<<<<<]>[-]>>[+++++++++++++++++++++++++++++ +++++++++++++++++++.[-]]<[++++++++++++++++++++++++++++++++++++++++++++++++.[-]]< <<++++++++++++++++++++++++++++++++++++++++++++++++.[-]<<<<<<<.>.>>[>>+<<-]>[>+<< +>-]>[<+>-]<<<-]<<++...";
        let entry = Hash::type_hash(["main"]);

        b.iter(|| {
            let value = vm.call(entry, (program, 0)).expect("failed call");
            let out = io.drain_utf8();
            assert_eq!(
                out.as_deref(),
                Ok("1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 121, 98, 219, ...")
            );
            value
        });
    });

    let (mut vm, io) = make_vm().unwrap();

    b.bench_function("brainfuck_loopity", |b| {
        // Just a program that runs a lot of instructions
        let program = ">+[>++>+++[-<]>>]+";
        let entry = Hash::type_hash(["main"]);

        b.iter(|| {
            let value = vm.call(entry, (program, 5)).expect("failed call");
            let out = io.drain_utf8();
            assert_eq!(out.as_deref(), Ok(""));
            value
        });
    });
}

fn make_vm() -> Result<(Vm, CaptureIo)> {
    Ok(rune_vm_capture! {
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

            fn get(self) {
                self.tape[self.pos]
            }

            fn inc(self, x) {
                self.tape[self.pos] = (self.tape[self.pos] + x) % 256;

                if self.tape[self.pos] < 0 {
                    self.tape[self.pos] = self.tape[self.pos] + 256;
                }
            }

            fn mov(self, x) {
                self.pos += x;

                while self.pos >= self.tape.len() {
                    self.tape.push(0);
                }
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
                        let c = char::from_i64(tape.get()).expect("A valid char");
                        print!("{}", c);
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
                let op = match c {
                    '+' => Op::Inc(1),
                    '-' => Op::Inc(-1),
                    '>' => Op::Move(1),
                    '<' => Op::Move(-1),
                    '.' => Op::Print,
                    '[' => Op::Loop(parse(it)),
                    ',' => Op::Input,
                    ']' => break,
                    _ => continue,
                };

                buf.push(op);
            }

            buf
        }

        struct Program {
            ops,
            inputs
        }

        impl Program {
            fn new(code, inputs) {
                Program { ops: parse(code), inputs }
            }

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
