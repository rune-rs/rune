use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::{
    traits::{Compile, Resolve as _},
    CompileError,
};
use runestick::{CompileMetaKind, Hash, Inst, Item, Span};

/// Compile a literal object.
impl Compile<(&ast::LitObject, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_object, needs): (&ast::LitObject, Needs)) -> CompileResult<()> {
        let span = lit_object.span();
        log::trace!("LitObject => {:?} {:?}", self.source.source(span), needs);

        if !needs.value() && lit_object.is_const() {
            // Don't encode unecessary literals.
            return Ok(());
        }

        let mut keys = Vec::new();
        let mut check_keys = Vec::new();
        let mut keys_dup = HashMap::new();

        for assign in &lit_object.assignments {
            let span = assign.span();
            let key = assign
                .key
                .resolve(&self.storage, &*self.source)?
                .to_string();
            keys.push(key.clone());
            check_keys.push((key.clone(), assign.key.span()));

            if let Some(existing) = keys_dup.insert(key, span) {
                return Err(CompileError::DuplicateObjectKey {
                    span,
                    existing,
                    object: span,
                });
            }
        }

        for assign in lit_object.assignments.iter() {
            let span = assign.span();

            if let Some((_, expr)) = &assign.assign {
                self.compile((expr, Needs::Value))?;

                // Evaluate the expressions one by one, then pop them to cause any
                // side effects (without creating an object).
                if !needs.value() {
                    self.asm.push(Inst::Pop, span);
                }
            } else {
                let key = assign.key.resolve(&self.storage, &*self.source)?;
                let var = self.scopes.get_var(&*key, self.visitor, span)?;

                if needs.value() {
                    var.copy(&mut self.asm, span, format!("name `{}`", key));
                }
            }
        }

        // No need to encode an object since the value is not needed.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        let slot = self.unit.borrow_mut().new_static_object_keys(&keys)?;

        match &lit_object.ident {
            ast::LitObjectIdent::Named(path) => {
                let item = self.convert_path_to_item(path)?;

                let meta = match self.lookup_meta(&item, path.span())? {
                    Some(meta) => meta,
                    None => {
                        return Err(CompileError::MissingType { span, item });
                    }
                };

                match &meta.kind {
                    CompileMetaKind::Struct { object, .. } => {
                        check_object_fields(
                            object.fields.as_ref(),
                            check_keys,
                            span,
                            &object.item,
                        )?;

                        let hash = Hash::type_hash(&object.item);
                        self.asm.push(Inst::TypedObject { hash, slot }, span);
                    }
                    CompileMetaKind::StructVariant {
                        enum_item, object, ..
                    } => {
                        check_object_fields(
                            object.fields.as_ref(),
                            check_keys,
                            span,
                            &object.item,
                        )?;

                        let enum_hash = Hash::type_hash(enum_item);
                        let hash = Hash::type_hash(&object.item);

                        self.asm.push(
                            Inst::VariantObject {
                                enum_hash,
                                hash,
                                slot,
                            },
                            span,
                        );
                    }
                    _ => {
                        return Err(CompileError::UnsupportedLitObject {
                            span,
                            item: meta.item().clone(),
                        });
                    }
                };
            }
            ast::LitObjectIdent::Anonymous(..) => {
                self.asm.push(Inst::Object { slot }, span);
            }
        }

        Ok(())
    }
}

fn check_object_fields(
    fields: Option<&HashSet<String>>,
    check_keys: Vec<(String, Span)>,
    span: Span,
    item: &Item,
) -> CompileResult<()> {
    let mut fields = match fields {
        Some(fields) => fields.clone(),
        None => {
            return Err(CompileError::MissingType {
                span,
                item: item.clone(),
            });
        }
    };

    for (field, span) in check_keys {
        if !fields.remove(&field) {
            return Err(CompileError::LitObjectNotField {
                span,
                field,
                item: item.clone(),
            });
        }
    }

    if let Some(field) = fields.into_iter().next() {
        return Err(CompileError::LitObjectMissingField {
            span,
            field,
            item: item.clone(),
        });
    }

    Ok(())
}
