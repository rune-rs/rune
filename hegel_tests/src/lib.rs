use core::mem::replace;

use rune::runtime::FromValue;
use rune::sync::Arc;
use rune::{Context, Diagnostics, Hash, Options, Source, Sources, Vm};

/// Outcome of evaluating a source that isn't a plain success.
#[derive(Debug)]
pub enum EvalError {
    Compile,
    Vm,
    Convert,
}

/// Compile `source` as a script and run it, returning the top-level value.
pub fn eval_result<T>(context: &Context, source: &str) -> Result<T, EvalError>
where
    T: FromValue,
{
    let mut sources = Sources::new();
    sources
        .insert(Source::memory(source).map_err(|_| EvalError::Compile)?)
        .map_err(|_| EvalError::Compile)?;

    let mut diagnostics = Diagnostics::new();
    let mut options = Options::default();
    options.script(true);

    let unit = rune::prepare(&mut sources)
        .with_context(context)
        .with_diagnostics(&mut diagnostics)
        .with_options(&options)
        .build()
        .map_err(|_| EvalError::Compile)?;
    let unit = Arc::try_new(unit).map_err(|_| EvalError::Compile)?;

    let runtime = Arc::try_new(context.runtime().map_err(|_| EvalError::Compile)?)
        .map_err(|_| EvalError::Compile)?;
    let mut vm = Vm::new(runtime, unit);

    let value = vm.call(Hash::EMPTY, ()).map_err(|_| EvalError::Vm)?;
    rune::runtime::from_value::<T>(value).map_err(|_| EvalError::Convert)
}

/// Compile `source`, discarding the result: the property is that compilation
/// returns instead of panicking, whatever the input.
pub fn try_compile(context: &Context, source: &str, script: bool) {
    let Ok(source) = Source::new("main", source) else {
        return;
    };

    let mut sources = Sources::new();
    sources.insert(source).expect("insert source");

    let mut diagnostics = Diagnostics::new();
    let mut options = Options::default();
    if script {
        options.script(true);
    }

    drop(
        rune::prepare(&mut sources)
            .with_context(context)
            .with_diagnostics(&mut diagnostics)
            .with_options(&options)
            .build(),
    );
}

/// Format `source`, returning the output if it could be formatted.
///
/// A source which cannot be laid out is not interesting here - the properties
/// below are about what the formatter does when it does produce something.
pub fn try_format(source: &str) -> Option<String> {
    let source = Source::new("main", source).ok()?;

    let mut sources = Sources::new();
    sources.insert(source).expect("insert source");

    let mut diagnostics = Diagnostics::new();

    let files = rune::fmt::prepare(&sources)
        .with_diagnostics(&mut diagnostics)
        .format()
        .ok()?;

    let (_, output) = files.into_iter().next()?;
    Some(output.into_std())
}

use hegel::generators;

#[derive(Debug, Clone, Copy)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl ArithOp {
    fn symbol(self) -> &'static str {
        match self {
            ArithOp::Add => "+",
            ArithOp::Sub => "-",
            ArithOp::Mul => "*",
            ArithOp::Div => "/",
            ArithOp::Rem => "%",
        }
    }

    fn apply(self, a: i64, b: i64) -> Option<i64> {
        match self {
            ArithOp::Add => a.checked_add(b),
            ArithOp::Sub => a.checked_sub(b),
            ArithOp::Mul => a.checked_mul(b),
            ArithOp::Div => a.checked_div(b),
            ArithOp::Rem => a.checked_rem(b),
        }
    }
}

#[derive(Debug)]
pub enum Expr {
    Lit(i64),
    Neg(Box<Expr>),
    Bin(ArithOp, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum RefError {
    Arith(ArithOp),
    NegOverflow,
    NotAnInteger,
    NotABoolean,
}

pub const ALL_OPS: [ArithOp; 5] = [
    ArithOp::Add,
    ArithOp::Sub,
    ArithOp::Mul,
    ArithOp::Div,
    ArithOp::Rem,
];

pub fn literal_gen() -> hegel::generators::BoxedGenerator<'static, i64> {
    use hegel::generators::Generator;
    hegel::one_of!(
        generators::integers::<i64>().min_value(-8).max_value(8),
        generators::integers::<i64>(),
        generators::sampled_from(vec![
            i64::MIN,
            i64::MIN + 1,
            -1,
            0,
            1,
            2,
            i64::MAX - 1,
            i64::MAX
        ]),
    )
    .boxed()
}

// Callers keep `depth` small so that a case stays quick to compile: nesting is
// walked over an explicit stack and bounded by the `max-depth` option rather
// than by the native stack, so what is generated here is nowhere near it.
pub fn expr(
    depth: u32,
    allow_neg: bool,
    ops: Vec<ArithOp>,
) -> hegel::generators::BoxedGenerator<'static, Expr> {
    use hegel::generators::Generator;

    let leaf = literal_gen().map(Expr::Lit).boxed();
    if depth == 0 {
        return leaf;
    }

    let bin = hegel::tuples!(
        generators::sampled_from(ops.clone()),
        expr(depth - 1, allow_neg, ops.clone()),
        expr(depth - 1, allow_neg, ops.clone())
    )
    .map(|(op, lhs, rhs)| Expr::Bin(op, Box::new(lhs), Box::new(rhs)))
    .boxed();

    if allow_neg {
        let neg = expr(depth - 1, allow_neg, ops.clone())
            .map(|e| Expr::Neg(Box::new(e)))
            .boxed();
        hegel::one_of!(leaf, neg, bin).boxed()
    } else {
        hegel::one_of!(leaf, bin).boxed()
    }
}

pub fn eval_reference(expr: &Expr) -> Result<i64, RefError> {
    match expr {
        Expr::Lit(n) => Ok(*n),
        Expr::Neg(inner) => eval_reference(inner)?
            .checked_neg()
            .ok_or(RefError::NegOverflow),
        Expr::Bin(op, lhs, rhs) => {
            let lhs = eval_reference(lhs)?;
            let rhs = eval_reference(rhs)?;
            op.apply(lhs, rhs).ok_or(RefError::Arith(*op))
        }
    }
}

pub fn render(expr: &Expr, out: &mut String) {
    match expr {
        Expr::Lit(n) => {
            out.push('(');
            out.push_str(&n.to_string());
            out.push(')');
        }
        Expr::Neg(inner) => {
            out.push_str("(-");
            render(inner, out);
            out.push(')');
        }
        Expr::Bin(op, lhs, rhs) => {
            out.push('(');
            render(lhs, out);
            out.push(' ');
            out.push_str(op.symbol());
            out.push(' ');
            render(rhs, out);
            out.push(')');
        }
    }
}

/// How many variables of each kind a generated program has.
pub const VARS: usize = 3;

/// The variables a generated program runs with.
///
/// Integers and booleans are kept apart so that a variable always holds the
/// kind of value the program expects to find in it, which is what lets the
/// reference below say what the program means without having to type-check it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vars {
    ints: [i64; VARS],
    bools: [bool; VARS],
}

/// What a generated program evaluates to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    Int(i64),
    Bool(bool),
}

/// A generated program.
///
/// Every shape here is one whose meaning is unambiguous, since the reference
/// has to agree with the machine exactly and a shape whose meaning is subtle
/// would report the reference being wrong as a bug in the machine.
///
/// Arithmetic is deliberately shallow: what this is for is the way values are
/// moved between registers as control flows through a program, which is what
/// [`Program::Match`], [`Program::Cond`] and the rest exercise. The edges of
/// arithmetic itself have a property of their own.
#[derive(Debug)]
pub enum Program {
    Int(i64),
    Bool(bool),
    VarInt(usize),
    VarBool(usize),
    Bin(ArithOp, Box<Program>, Box<Program>),
    Neg(Box<Program>),
    /// `a < b` over integers.
    Lt(Box<Program>, Box<Program>),
    /// `a == b`, over integers or over booleans.
    EqInt(Box<Program>, Box<Program>),
    EqBool(Box<Program>, Box<Program>),
    /// `a && b` and `a || b`, which short-circuit.
    Cond(bool, Box<Program>, Box<Program>),
    Not(Box<Program>),
    /// `if <cond> { a } else { b }`.
    If(Box<Program>, Box<Program>, Box<Program>),
    /// `match <int> { 0 => a, _ => b }`.
    Match(Box<Program>, Box<Program>, Box<Program>),
    /// `{ let v<n> = <value>; <body> }`, which puts the variable back on the
    /// way out.
    LetInt(usize, Box<Program>, Box<Program>),
    LetBool(usize, Box<Program>, Box<Program>),
    /// `{ v<n> = <value>; <body> }`, which does not.
    AssignInt(usize, Box<Program>, Box<Program>),
    AssignBool(usize, Box<Program>, Box<Program>),
    /// `'l: { if <cond> { break 'l <a>; } <b> }`.
    Break(Box<Program>, Box<Program>, Box<Program>),
    /// The same where every path out of the block breaks, so the block itself
    /// never falls through to an end of its own.
    BreakAlways(Box<Program>, Box<Program>, Box<Program>),
    /// `{ let v<n> = 0; for _ in 0..<count> { v<n> = <step>; } <body> }`.
    For(usize, u8, Box<Program>, Box<Program>),
    /// The same over a `while`, counted down by a variable the step cannot
    /// reach so that it always terminates.
    While(usize, Box<Program>, Box<Program>),
    /// `(|v<n>| <body>)(<arg>)`, which captures by value.
    CallInt(usize, Box<Program>, Box<Program>),
    CallBool(usize, Box<Program>, Box<Program>),
    /// `{ let a = [<a>, <b>]; a[<0 or 1>] }`, which evaluates both.
    Index(bool, Box<Program>, Box<Program>),
}

/// How many times the loops a generated program contains go around.
const LOOP_COUNT: u8 = 3;

fn int(outcome: Outcome) -> Result<i64, RefError> {
    match outcome {
        Outcome::Int(n) => Ok(n),
        Outcome::Bool(..) => Err(RefError::NotAnInteger),
    }
}

fn boolean(outcome: Outcome) -> Result<bool, RefError> {
    match outcome {
        Outcome::Bool(b) => Ok(b),
        Outcome::Int(..) => Err(RefError::NotABoolean),
    }
}

/// Evaluate a generated program, saying what it means.
pub fn run_reference(program: &Program, vars: &mut Vars) -> Result<Outcome, RefError> {
    let outcome = match program {
        Program::Int(n) => Outcome::Int(*n),
        Program::Bool(b) => Outcome::Bool(*b),
        Program::VarInt(n) => Outcome::Int(vars.ints[*n]),
        Program::VarBool(n) => Outcome::Bool(vars.bools[*n]),
        Program::Bin(op, a, b) => {
            let a = int(run_reference(a, vars)?)?;
            let b = int(run_reference(b, vars)?)?;
            Outcome::Int(op.apply(a, b).ok_or(RefError::Arith(*op))?)
        }
        Program::Neg(a) => {
            let a = int(run_reference(a, vars)?)?;
            Outcome::Int(a.checked_neg().ok_or(RefError::NegOverflow)?)
        }
        Program::Lt(a, b) => {
            let a = int(run_reference(a, vars)?)?;
            let b = int(run_reference(b, vars)?)?;
            Outcome::Bool(a < b)
        }
        Program::EqInt(a, b) => {
            let a = int(run_reference(a, vars)?)?;
            let b = int(run_reference(b, vars)?)?;
            Outcome::Bool(a == b)
        }
        Program::EqBool(a, b) => {
            let a = boolean(run_reference(a, vars)?)?;
            let b = boolean(run_reference(b, vars)?)?;
            Outcome::Bool(a == b)
        }
        Program::Cond(and, a, b) => {
            let a = boolean(run_reference(a, vars)?)?;

            // The right-hand side is only reached when the left-hand side does
            // not decide the answer, and what it does is only done then.
            if a != *and {
                Outcome::Bool(a)
            } else {
                Outcome::Bool(boolean(run_reference(b, vars)?)?)
            }
        }
        Program::Not(a) => Outcome::Bool(!boolean(run_reference(a, vars)?)?),
        Program::If(cond, a, b) => {
            if boolean(run_reference(cond, vars)?)? {
                run_reference(a, vars)?
            } else {
                run_reference(b, vars)?
            }
        }
        Program::Match(value, a, b) => {
            if int(run_reference(value, vars)?)? == 0 {
                run_reference(a, vars)?
            } else {
                run_reference(b, vars)?
            }
        }
        Program::LetInt(n, value, body) => {
            let value = int(run_reference(value, vars)?)?;
            let old = replace(&mut vars.ints[*n], value);
            let outcome = run_reference(body, vars);
            vars.ints[*n] = old;
            outcome?
        }
        Program::LetBool(n, value, body) => {
            let value = boolean(run_reference(value, vars)?)?;
            let old = replace(&mut vars.bools[*n], value);
            let outcome = run_reference(body, vars);
            vars.bools[*n] = old;
            outcome?
        }
        Program::AssignInt(n, value, body) => {
            vars.ints[*n] = int(run_reference(value, vars)?)?;
            run_reference(body, vars)?
        }
        Program::AssignBool(n, value, body) => {
            vars.bools[*n] = boolean(run_reference(value, vars)?)?;
            run_reference(body, vars)?
        }
        Program::Break(cond, a, b) | Program::BreakAlways(cond, a, b) => {
            if boolean(run_reference(cond, vars)?)? {
                run_reference(a, vars)?
            } else {
                run_reference(b, vars)?
            }
        }
        Program::For(n, count, step, body) => {
            let old = replace(&mut vars.ints[*n], 0);

            for _ in 0..*count {
                match run_reference(step, vars) {
                    Ok(value) => vars.ints[*n] = int(value)?,
                    Err(error) => {
                        vars.ints[*n] = old;
                        return Err(error);
                    }
                }
            }

            let outcome = run_reference(body, vars);
            vars.ints[*n] = old;
            outcome?
        }
        Program::While(n, step, body) => {
            let old = replace(&mut vars.ints[*n], 0);

            for _ in 0..LOOP_COUNT {
                match run_reference(step, vars) {
                    Ok(value) => vars.ints[*n] = int(value)?,
                    Err(error) => {
                        vars.ints[*n] = old;
                        return Err(error);
                    }
                }
            }

            let outcome = run_reference(body, vars);
            vars.ints[*n] = old;
            outcome?
        }
        Program::CallInt(n, arg, body) => {
            // A closure captures what it uses by value when it is made, which
            // is before the argument is evaluated.
            let captured = *vars;
            let arg = int(run_reference(arg, vars)?)?;

            // Nothing the closure does to what it captured is visible once it
            // has returned, but what the argument did is.
            let outer = *vars;
            let mut inner = captured;
            inner.ints[*n] = arg;
            let outcome = run_reference(body, &mut inner);
            *vars = outer;
            outcome?
        }
        Program::CallBool(n, arg, body) => {
            let captured = *vars;
            let arg = boolean(run_reference(arg, vars)?)?;

            let outer = *vars;
            let mut inner = captured;
            inner.bools[*n] = arg;
            let outcome = run_reference(body, &mut inner);
            *vars = outer;
            outcome?
        }
        Program::Index(second, a, b) => {
            let a = run_reference(a, vars)?;
            let b = run_reference(b, vars)?;

            if *second {
                b
            } else {
                a
            }
        }
    };

    Ok(outcome)
}

/// Write a generated program out as the source it stands for.
///
/// Everything is written in parentheses or braces: a block which is followed by
/// something starting with an operator would otherwise read as that operator
/// being applied to the block, which is a different program.
pub fn render_program(program: &Program, out: &mut String) {
    use core::fmt::Write;

    match program {
        Program::Int(n) => {
            let _ = write!(out, "({n})");
        }
        Program::Bool(b) => {
            let _ = write!(out, "({b})");
        }
        Program::VarInt(n) => {
            let _ = write!(out, "v{n}");
        }
        Program::VarBool(n) => {
            let _ = write!(out, "b{n}");
        }
        Program::Bin(op, a, b) => binary(out, op.symbol(), a, b),
        Program::Neg(a) => {
            out.push_str("(-");
            render_program(a, out);
            out.push(')');
        }
        Program::Lt(a, b) => binary(out, "<", a, b),
        Program::EqInt(a, b) | Program::EqBool(a, b) => binary(out, "==", a, b),
        Program::Cond(and, a, b) => binary(out, if *and { "&&" } else { "||" }, a, b),
        Program::Not(a) => {
            out.push_str("(!");
            render_program(a, out);
            out.push(')');
        }
        Program::If(cond, a, b) => {
            out.push_str("(if ");
            render_program(cond, out);
            out.push_str(" { ");
            render_program(a, out);
            out.push_str(" } else { ");
            render_program(b, out);
            out.push_str(" })");
        }
        Program::Match(value, a, b) => {
            out.push_str("(match ");
            render_program(value, out);
            out.push_str(" { 0 => ");
            render_program(a, out);
            out.push_str(", _ => ");
            render_program(b, out);
            out.push_str(" })");
        }
        Program::LetInt(n, value, body) => binding(out, &format!("let v{n}"), value, body),
        Program::LetBool(n, value, body) => binding(out, &format!("let b{n}"), value, body),
        Program::AssignInt(n, value, body) => binding(out, &format!("v{n}"), value, body),
        Program::AssignBool(n, value, body) => binding(out, &format!("b{n}"), value, body),
        Program::Break(cond, a, b) => {
            out.push_str("('l: { if ");
            render_program(cond, out);
            out.push_str(" { break 'l ");
            render_program(a, out);
            out.push_str("; } ");
            render_program(b, out);
            out.push_str(" })");
        }
        Program::BreakAlways(cond, a, b) => {
            out.push_str("('l: { if ");
            render_program(cond, out);
            out.push_str(" { break 'l ");
            render_program(a, out);
            out.push_str("; } break 'l ");
            render_program(b, out);
            out.push_str("; })");
        }
        Program::For(n, count, step, body) => {
            let _ = write!(out, "{{ let v{n} = 0; for _ in 0..{count} {{ v{n} = ");
            render_program(step, out);
            out.push_str("; } ");
            render_program(body, out);
            out.push_str(" }");
        }
        Program::While(n, step, body) => {
            let _ = write!(
                out,
                "{{ let v{n} = 0; let counter = {LOOP_COUNT}; \
                 while counter > 0 {{ v{n} = "
            );
            render_program(step, out);
            out.push_str("; counter -= 1; } ");
            render_program(body, out);
            out.push_str(" }");
        }
        Program::CallInt(n, arg, body) => closure(out, &format!("v{n}"), arg, body),
        Program::CallBool(n, arg, body) => closure(out, &format!("b{n}"), arg, body),
        Program::Index(second, a, b) => {
            out.push_str("{ let items = [");
            render_program(a, out);
            out.push_str(", ");
            render_program(b, out);
            let _ = write!(out, "]; items[{}] }}", usize::from(*second));
        }
    }
}

fn binary(out: &mut String, op: &str, a: &Program, b: &Program) {
    out.push('(');
    render_program(a, out);
    out.push(' ');
    out.push_str(op);
    out.push(' ');
    render_program(b, out);
    out.push(')');
}

fn binding(out: &mut String, target: &str, value: &Program, body: &Program) {
    out.push_str("{ ");
    out.push_str(target);
    out.push_str(" = ");
    render_program(value, out);
    out.push_str("; ");
    render_program(body, out);
    out.push_str(" }");
}

fn closure(out: &mut String, parameter: &str, arg: &Program, body: &Program) {
    out.push_str("((|");
    out.push_str(parameter);
    out.push_str("| ");
    render_program(body, out);
    out.push_str(")(");
    render_program(arg, out);
    out.push_str("))");
}

/// Write the whole of a generated program, variables and all.
///
/// What the program leaves in its variables is handed back along with what it
/// evaluated to, since a construct which writes into a variable it was only
/// supposed to read is not visible in the value alone: the program would have
/// to go on and read that variable for it to show.
pub fn render_source(program: &Program) -> String {
    use core::fmt::Write;

    let mut out = String::new();

    for n in 0..VARS {
        let _ = write!(out, "let v{n} = 0; let b{n} = false; ");
    }

    out.push_str("let result = ");
    render_program(program, &mut out);
    out.push_str("; (result, [");

    for n in 0..VARS {
        if n > 0 {
            out.push_str(", ");
        }

        let _ = write!(out, "v{n}");
    }

    out.push_str("], [");

    for n in 0..VARS {
        if n > 0 {
            out.push_str(", ");
        }

        let _ = write!(out, "b{n}");
    }

    out.push_str("])");
    out
}

impl Vars {
    /// The integers the program left behind.
    pub fn ints(&self) -> &[i64] {
        &self.ints
    }

    /// The booleans the program left behind.
    pub fn bools(&self) -> &[bool] {
        &self.bools
    }
}

/// The literals a generated program is built from.
///
/// These stay small so that a program is rejected for overflowing only rarely:
/// what the edges of arithmetic do has a property of its own, and letting them
/// in here would throw away most of the programs before they were ever run.
fn program_literal() -> hegel::generators::BoxedGenerator<'static, i64> {
    use hegel::generators::Generator;
    generators::integers::<i64>()
        .min_value(-4)
        .max_value(16)
        .boxed()
}

fn variable() -> hegel::generators::BoxedGenerator<'static, usize> {
    use hegel::generators::Generator;
    generators::integers::<usize>()
        .max_value(VARS - 1)
        .boxed()
}

/// Generate a program which evaluates to an integer, or to a boolean.
///
/// The two are generated apart so that what comes out is a program which runs:
/// a generator which produced `1 && true` would spend every case on the machine
/// reporting a type error rather than on what the program means.
pub fn program(depth: u32, boolean: bool) -> hegel::generators::BoxedGenerator<'static, Program> {
    use hegel::generators::Generator;

    if depth == 0 {
        return if boolean {
            hegel::one_of!(
                generators::booleans().map(Program::Bool),
                variable().map(Program::VarBool),
            )
            .boxed()
        } else {
            hegel::one_of!(
                program_literal().map(Program::Int),
                variable().map(Program::VarInt),
            )
            .boxed()
        };
    }

    let next = depth - 1;

    // The shapes which hand back whatever their body hands back, so they are
    // generated for either kind.
    let cond = hegel::tuples!(
        program(next, true),
        program(next, boolean),
        program(next, boolean)
    )
    .map(|(c, a, b)| Program::If(Box::new(c), Box::new(a), Box::new(b)))
    .boxed();

    let match_ = hegel::tuples!(
        program(next, false),
        program(next, boolean),
        program(next, boolean)
    )
    .map(|(v, a, b)| Program::Match(Box::new(v), Box::new(a), Box::new(b)))
    .boxed();

    let let_int = hegel::tuples!(variable(), program(next, false), program(next, boolean))
        .map(|(n, v, b)| Program::LetInt(n, Box::new(v), Box::new(b)))
        .boxed();

    let let_bool = hegel::tuples!(variable(), program(next, true), program(next, boolean))
        .map(|(n, v, b)| Program::LetBool(n, Box::new(v), Box::new(b)))
        .boxed();

    let assign_int = hegel::tuples!(variable(), program(next, false), program(next, boolean))
        .map(|(n, v, b)| Program::AssignInt(n, Box::new(v), Box::new(b)))
        .boxed();

    let assign_bool = hegel::tuples!(variable(), program(next, true), program(next, boolean))
        .map(|(n, v, b)| Program::AssignBool(n, Box::new(v), Box::new(b)))
        .boxed();

    let break_ = hegel::tuples!(
        program(next, true),
        program(next, boolean),
        program(next, boolean)
    )
    .map(|(c, a, b)| Program::Break(Box::new(c), Box::new(a), Box::new(b)))
    .boxed();

    let break_always = hegel::tuples!(
        program(next, true),
        program(next, boolean),
        program(next, boolean)
    )
    .map(|(c, a, b)| Program::BreakAlways(Box::new(c), Box::new(a), Box::new(b)))
    .boxed();

    let for_ = hegel::tuples!(
        variable(),
        generators::integers::<u8>().max_value(LOOP_COUNT),
        program(next, false),
        program(next, boolean)
    )
    .map(|(n, count, s, b)| Program::For(n, count, Box::new(s), Box::new(b)))
    .boxed();

    let while_ = hegel::tuples!(variable(), program(next, false), program(next, boolean))
        .map(|(n, s, b)| Program::While(n, Box::new(s), Box::new(b)))
        .boxed();

    let call_int = hegel::tuples!(variable(), program(next, false), program(next, boolean))
        .map(|(n, a, b)| Program::CallInt(n, Box::new(a), Box::new(b)))
        .boxed();

    let call_bool = hegel::tuples!(variable(), program(next, true), program(next, boolean))
        .map(|(n, a, b)| Program::CallBool(n, Box::new(a), Box::new(b)))
        .boxed();

    let index = hegel::tuples!(
        generators::booleans(),
        program(next, boolean),
        program(next, boolean)
    )
    .map(|(second, a, b)| Program::Index(second, Box::new(a), Box::new(b)))
    .boxed();

    let shared = hegel::one_of!(
        cond,
        match_,
        let_int,
        let_bool,
        assign_int,
        assign_bool,
        break_,
        break_always,
        for_,
        while_,
        call_int,
        call_bool,
        index,
    )
    .boxed();

    if boolean {
        let lt = hegel::tuples!(program(next, false), program(next, false))
            .map(|(a, b)| Program::Lt(Box::new(a), Box::new(b)))
            .boxed();

        let eq_int = hegel::tuples!(program(next, false), program(next, false))
            .map(|(a, b)| Program::EqInt(Box::new(a), Box::new(b)))
            .boxed();

        let eq_bool = hegel::tuples!(program(next, true), program(next, true))
            .map(|(a, b)| Program::EqBool(Box::new(a), Box::new(b)))
            .boxed();

        let conditional = hegel::tuples!(
            generators::booleans(),
            program(next, true),
            program(next, true)
        )
        .map(|(and, a, b)| Program::Cond(and, Box::new(a), Box::new(b)))
        .boxed();

        let not = program(next, true)
            .map(|a| Program::Not(Box::new(a)))
            .boxed();

        return hegel::one_of!(
            program(0, true),
            lt,
            eq_int,
            eq_bool,
            conditional,
            not,
            shared,
        )
        .boxed();
    }

    let bin = hegel::tuples!(
        generators::sampled_from(vec![ArithOp::Add, ArithOp::Sub, ArithOp::Mul]),
        program(next, false),
        program(next, false)
    )
    .map(|(op, a, b)| Program::Bin(op, Box::new(a), Box::new(b)))
    .boxed();

    let neg = program(next, false)
        .map(|a| Program::Neg(Box::new(a)))
        .boxed();

    hegel::one_of!(program(0, false), bin, neg, shared).boxed()
}
