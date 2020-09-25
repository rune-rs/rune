use crate::compiler::compile::prelude::*;

/// Call an async block.
impl Compile<(&ConstValue, Span)> for Compiler<'_> {
    fn compile(&mut self, (const_value, span): (&ConstValue, Span)) -> CompileResult<()> {
        match const_value {
            ConstValue::Unit => {
                self.asm.push(Inst::unit(), span);
            }
            ConstValue::Byte(b) => {
                self.asm.push(Inst::byte(*b), span);
            }
            ConstValue::Char(c) => {
                self.asm.push(Inst::char(*c), span);
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
                let slot = self.unit.new_static_string(&s)?;
                self.asm.push(Inst::String { slot }, span);
            }
            ConstValue::Bytes(b) => {
                let slot = self.unit.new_static_bytes(&*b)?;
                self.asm.push(Inst::Bytes { slot }, span);
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
            ConstValue::Object(object) => {
                let mut entries = object.iter().collect::<Vec<_>>();
                entries.sort_by_key(|k| k.0);

                for (_, value) in &entries {
                    self.compile((*value, span))?;
                }

                let slot = self
                    .unit
                    .new_static_object_keys(entries.iter().map(|e| e.0))?;

                self.asm.push(Inst::Object { slot }, span);
            }
        }

        Ok(())
    }
}
