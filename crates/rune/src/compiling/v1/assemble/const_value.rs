use crate::compiling::v1::assemble::prelude::*;

/// Assemble a constant value.
impl AssembleConst for ConstValue {
    fn assemble_const(&self, c: &mut Compiler<'_>, needs: Needs, span: Span) -> CompileResult<()> {
        use num::ToPrimitive as _;

        if !needs.value() {
            c.diagnostics.not_used(c.source_id, span, c.context());
            return Ok(());
        }

        match self {
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
                let slot = c.unit.new_static_string(span, s)?;
                c.asm.push(Inst::String { slot }, span);
            }
            ConstValue::StaticString(s) => {
                let slot = c.unit.new_static_string(span, s.as_ref())?;
                c.asm.push(Inst::String { slot }, span);
            }
            ConstValue::Bytes(b) => {
                let slot = c.unit.new_static_bytes(span, &*b)?;
                c.asm.push(Inst::Bytes { slot }, span);
            }
            ConstValue::Option(option) => match option {
                Some(value) => {
                    value.assemble_const(c, Needs::Value, span)?;
                    c.asm.push(
                        Inst::Variant {
                            variant: InstVariant::Some,
                        },
                        span,
                    );
                }
                None => {
                    c.asm.push(
                        Inst::Variant {
                            variant: InstVariant::None,
                        },
                        span,
                    );
                }
            },
            ConstValue::Vec(vec) => {
                for value in vec.iter() {
                    value.assemble_const(c, Needs::Value, span)?;
                }

                c.asm.push(Inst::Vec { count: vec.len() }, span);
            }
            ConstValue::Tuple(tuple) => {
                for value in tuple.iter() {
                    value.assemble_const(c, Needs::Value, span)?;
                }

                c.asm.push(Inst::Tuple { count: tuple.len() }, span);
            }
            ConstValue::Object(object) => {
                let mut entries = object.iter().collect::<Vec<_>>();
                entries.sort_by_key(|k| k.0);

                for (_, value) in entries.iter().copied() {
                    value.assemble_const(c, Needs::Value, span)?;
                }

                let slot = c
                    .unit
                    .new_static_object_keys_iter(span, entries.iter().map(|e| e.0))?;

                c.asm.push(Inst::Object { slot }, span);
            }
        }

        Ok(())
    }
}
