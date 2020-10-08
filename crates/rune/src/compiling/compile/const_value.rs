use crate::compiling::compile::prelude::*;

/// Call an async block.
impl Compile2 for (&ConstValue, Span) {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        use num::ToPrimitive as _;

        let (const_value, span) = *self;

        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            return Ok(());
        }

        match const_value {
            ConstValue::Unit => {
                c.asm.push(Inst::unit(), span);
            }
            ConstValue::Byte(b) => {
                c.asm.push(Inst::byte(*b), span);
            }
            ConstValue::Char(ch) => {
                c.asm.push(Inst::char(*ch), span);
            }
            ConstValue::Integer(n) => {
                let n = match n.to_i64() {
                    Some(n) => n,
                    None => {
                        return Err(CompileError::new(
                            span,
                            ParseErrorKind::BadNumberOutOfBounds,
                        ));
                    }
                };

                c.asm.push(Inst::integer(n), span);
            }
            ConstValue::Float(n) => {
                c.asm.push(Inst::float(*n), span);
            }
            ConstValue::Bool(b) => {
                c.asm.push(Inst::bool(*b), span);
            }
            ConstValue::String(s) => {
                let slot = c.unit.new_static_string(span, &s)?;
                c.asm.push(Inst::String { slot }, span);
            }
            ConstValue::Bytes(b) => {
                let slot = c.unit.new_static_bytes(span, &*b)?;
                c.asm.push(Inst::Bytes { slot }, span);
            }
            ConstValue::Vec(vec) => {
                for value in vec.iter() {
                    (value, span).compile2(c, Needs::Value)?;
                }

                c.asm.push(Inst::Vec { count: vec.len() }, span);
            }
            ConstValue::Tuple(tuple) => {
                for value in tuple.iter() {
                    (value, span).compile2(c, Needs::Value)?;
                }

                c.asm.push(Inst::Tuple { count: tuple.len() }, span);
            }
            ConstValue::Object(object) => {
                let mut entries = object.iter().collect::<Vec<_>>();
                entries.sort_by_key(|k| k.0);

                for (_, value) in entries.iter().copied() {
                    (value, span).compile2(c, Needs::Value)?;
                }

                let slot = c
                    .unit
                    .new_static_object_keys(span, entries.iter().map(|e| e.0))?;

                c.asm.push(Inst::Object { slot }, span);
            }
        }

        Ok(())
    }
}
