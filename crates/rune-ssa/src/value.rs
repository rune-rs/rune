use crate::{Assign, ConstId, Phi, Var};
use std::fmt;

/// A single abstract machine instruction.
#[derive(Debug, Clone)]
pub enum Value {
    /// A numerated input.
    Input(usize),
    /// An instruction to load a constant as a value.
    Const(ConstId),
    /// A value directly references a different value by its assignment.
    Assign(Assign),
    /// A phony use node, indicating what assignments flow into this.
    Phi(Phi),
    /// Compute `!arg`.
    Not(Assign),
    /// Compute `lhs + rhs`.
    Add(Assign, Assign),
    /// Compute `lhs - rhs`.
    Sub(Assign, Assign),
    /// Compute `lhs / rhs`.
    Div(Assign, Assign),
    /// Compute `lhs * rhs`.
    Mul(Assign, Assign),
    /// Compare if `lhs < rhs`.
    CmpLt(Assign, Assign),
    /// Compare if `lhs <= rhs`.
    CmpLte(Assign, Assign),
    /// Compare if `lhs == rhs`.
    CmpEq(Assign, Assign),
    /// Compare if `lhs > rhs`.
    CmpGt(Assign, Assign),
    /// Compare if `lhs >= rhs`.
    CmpGte(Assign, Assign),
}

impl Value {
    /// Dump diagnostical information on an instruction.
    pub fn dump(&self) -> InstDump<'_> {
        InstDump(self)
    }

    /// Test if value does not refer to the given var.
    pub fn is_var(&self, var: Var) -> bool {
        matches!(self, Self::Assign(v) if v.var() == var)
    }
}

pub struct InstDump<'a>(&'a Value);

impl fmt::Display for InstDump<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Value::Input(n) => {
                write!(f, "input {}", n)?;
            }
            Value::Const(id) => {
                write!(f, "{}", id)?;
            }
            Value::Assign(assign) => {
                write!(f, "{}", assign)?;
            }
            Value::Phi(phi) => {
                write!(f, "{}", phi)?;
            }
            Value::Not(var) => {
                write!(f, "not {}", var)?;
            }
            Value::Add(lhs, rhs) => {
                write!(f, "add {}, {}", lhs, rhs)?;
            }
            Value::Sub(lhs, rhs) => {
                write!(f, "sub {}, {}", lhs, rhs)?;
            }
            Value::Div(lhs, rhs) => {
                write!(f, "div {}, {}", lhs, rhs)?;
            }
            Value::Mul(lhs, rhs) => {
                write!(f, "mul {}, {}", lhs, rhs)?;
            }
            Value::CmpLt(lhs, rhs) => {
                write!(f, "lt {}, {}", lhs, rhs)?;
            }
            Value::CmpLte(lhs, rhs) => {
                write!(f, "lte {}, {}", lhs, rhs)?;
            }
            Value::CmpEq(lhs, rhs) => {
                write!(f, "eq {}, {}", lhs, rhs)?;
            }
            Value::CmpGt(lhs, rhs) => {
                write!(f, "gt {}, {}", lhs, rhs)?;
            }
            Value::CmpGte(lhs, rhs) => {
                write!(f, "gte {}, {}", lhs, rhs)?;
            }
        }

        Ok(())
    }
}
