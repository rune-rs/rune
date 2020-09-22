use crate::compile::prelude::*;

/// Call an async block.
impl Compile<(&ConstValue, Span)> for Compiler<'_> {
    fn compile(&mut self, (const_value, span): (&ConstValue, Span)) -> CompileResult<()> {
        match const_value {
            ConstValue::Unit => {
                self.asm.push(Inst::unit(), span);
            }
            ConstValue::Integer(n) => {
                self.asm.push(Inst::integer(*n), span);
            }
            ConstValue::Float(n) => {
                self.asm.push(Inst::float(*n), span);
            }
            ConstValue::Bool(b) => {
                self.asm.push(Inst::bool(*b), span);
            }
            ConstValue::String(s) => {
                let slot = self.unit.borrow_mut().new_static_string(&s)?;
                self.asm.push(Inst::String { slot }, span);
            }
            ConstValue::Vec(vec) => {
                for value in vec.iter() {
                    self.compile((value, span))?;
                }

                self.asm.push(Inst::Vec { count: vec.len() }, span);
            }
            ConstValue::Tuple(tuple) => {
                for value in tuple.iter() {
                    self.compile((value, span))?;
                }

                self.asm.push(Inst::Tuple { count: tuple.len() }, span);
            }
        }

        Ok(())
    }
}
