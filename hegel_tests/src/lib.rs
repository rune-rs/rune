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

// Callers keep `depth` small: rune aborts the process on deeply nested input
// (#1040), below hegel's own depth cap, so generated expressions must stay shallow.
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
