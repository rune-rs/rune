use crate::collections::{HashMap, HashSet};
use crate::compiling::compile::prelude::*;

/// Compile a literal object.
impl Compile2 for ast::LitObject {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("LitObject => {:?} {:?}", c.source.source(span), needs);

        let mut keys = Vec::new();
        let mut check_keys = Vec::new();
        let mut keys_dup = HashMap::new();

        for (assign, _) in &self.assignments {
            let span = assign.span();
            let key = assign.key.resolve(&c.storage, &*c.source)?.to_string();
            keys.push(key.clone());
            check_keys.push((key.clone(), assign.key.span()));

            if let Some(existing) = keys_dup.insert(key, span) {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::DuplicateObjectKey {
                        existing,
                        object: span,
                    },
                ));
            }
        }

        for (assign, _) in &self.assignments {
            let span = assign.span();

            if let Some((_, expr)) = &assign.assign {
                expr.compile2(c, Needs::Value)?;
            } else {
                let key = assign.key.resolve(&c.storage, &*c.source)?;
                let var = c.scopes.get_var(&*key, c.source_id, c.visitor, span)?;
                var.copy(&mut c.asm, span, format!("name `{}`", key));
            }
        }

        let slot = c.unit.new_static_object_keys(span, &keys)?;

        match &self.ident {
            ast::LitObjectIdent::Named(path) => {
                let named = c.convert_path_to_named(path)?;

                let meta = match c.lookup_meta(path.span(), &named)? {
                    Some(meta) => meta,
                    None => {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::MissingType {
                                item: named.item.clone(),
                            },
                        ));
                    }
                };

                match &meta.kind {
                    CompileMetaKind::UnitStruct { .. } => {
                        check_object_fields(Some(&HashSet::new()), check_keys, span, &meta.item)?;

                        let hash = Hash::type_hash(&meta.item);
                        c.asm.push(Inst::UnitStruct { hash }, span);
                    }
                    CompileMetaKind::Struct { object, .. } => {
                        check_object_fields(object.fields.as_ref(), check_keys, span, &meta.item)?;

                        let hash = Hash::type_hash(&meta.item);
                        c.asm.push(Inst::Struct { hash, slot }, span);
                    }
                    CompileMetaKind::StructVariant { object, .. } => {
                        check_object_fields(object.fields.as_ref(), check_keys, span, &meta.item)?;

                        let hash = Hash::type_hash(&meta.item);
                        c.asm.push(Inst::StructVariant { hash, slot }, span);
                    }
                    _ => {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedLitObject { meta },
                        ));
                    }
                };
            }
            ast::LitObjectIdent::Anonymous(..) => {
                c.asm.push(Inst::Object { slot }, span);
            }
        }

        // No need to encode an object since the value is not needed.
        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            c.asm.push(Inst::Pop, span);
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
            return Err(CompileError::new(
                span,
                CompileErrorKind::MissingType { item: item.clone() },
            ));
        }
    };

    for (field, span) in check_keys {
        if !fields.remove(&field) {
            return Err(CompileError::new(
                span,
                CompileErrorKind::LitObjectNotField {
                    field,
                    item: item.clone(),
                },
            ));
        }
    }

    if let Some(field) = fields.into_iter().next() {
        return Err(CompileError::new(
            span,
            CompileErrorKind::LitObjectMissingField {
                field,
                item: item.clone(),
            },
        ));
    }

    Ok(())
}
