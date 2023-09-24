prelude!();

use std::collections::BTreeMap;

struct DocVisitor {
    expected: BTreeMap<&'static str, Vec<&'static str>>,
    collected: BTreeMap<String, Vec<String>>,
}

impl compile::CompileVisitor for DocVisitor {
    fn visit_doc_comment(
        &mut self,
        _: &dyn Located,
        item: &Item,
        _: Hash,
        doc: &str,
    ) -> Result<(), compile::MetaError> {
        self.collected
            .entry(item.to_string())
            .or_default()
            .push(doc.to_string());
        Ok(())
    }

    fn visit_field_doc_comment(
        &mut self,
        _: &dyn Located,
        item: &Item,
        _: Hash,
        field: &str,
        doc: &str,
    ) -> Result<(), compile::MetaError> {
        self.collected
            .entry(format!("{item}.{field}"))
            .or_default()
            .push(doc.to_string());
        Ok(())
    }
}

impl DocVisitor {
    fn assert(&self) {
        for (&item, expected) in &self.expected {
            let against = if let Some(vec) = self.collected.get(item) {
                vec
            } else {
                let items = self
                    .collected
                    .keys()
                    .map(|item| item.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                panic!("missing documentation for item {item:?}, collected: {items}");
            };

            for (i, expected) in expected.iter().enumerate() {
                if let Some(collected) = against.get(i) {
                    assert_eq!(collected, expected, "mismatched docstring");
                } else {
                    panic!("missing docstrings, expected: {:?}", expected);
                }
            }

            if expected.len() < against.len() {
                let (_, extras) = against.split_at(expected.len());
                panic!("extra docstrings: {:?}", extras);
            }
        }

        if self.collected.len() > self.expected.len() {
            let vec = self
                .collected
                .keys()
                .filter(|it| !self.expected.contains_key(it.as_str()))
                .collect::<Vec<_>>();
            panic!("encountered more documented items than expected: {vec:?}");
        }
    }
}

macro_rules! expect_docs {
    ($($typename:literal => { $($docstr:literal)* })+) => {
        {
            let mut expected = BTreeMap::new();

            $(
            expected.insert($typename, vec![$($docstr),*]);
            )+

            DocVisitor {
                expected,
                collected: BTreeMap::new()
            }
        }
    };
}

#[test]
fn harvest_docs() -> Result<()> {
    let mut diagnostics = Diagnostics::new();
    let mut vis = expect_docs! {
        "{root}" => {
            " Mod/file doc.\n"
            " Multiline mod/file doc.\n         *  :)\n         "
        }
        "stuff" => { " Top-level function.\n" }
        "Struct" => {
            " Top-level struct.\n"
            " Second line!\n"
        }
        "Struct.a" => { " Struct field A.\n" }
        "Struct.b" => { " Struct field B.\n" }
        "Enum" => { "\n         * Top-level enum.\n         " }
        "Enum::A" => { " Enum variant A.\n" }
        "Enum::B" => { " Enum variant B.\n" }
        "Enum::B.a" => { " Enum struct variant B field A.\n" }
        "Enum::B.b" => { " Enum struct variant B field B.\n" }
        "CONSTANT" => { " Top-level constant.\n" }

        "module" => {
            " Top-level module.\n"
            " Also module doc.\n"
        }
        "module::Enum" => { " Module enum.\n" }
        "module::Enum::A" => { " Enum variant A.\n" }
        "module::Enum::B" => { " Enum variant B.\n" }

        "module::Module" => { " Module in a module.\n" }
        "module::Module::Enum" => { " Module enum.\n" }
        "module::Module::Enum::A" => { " Enum variant A.\n" }
        "module::Module::Enum::B" => { " Enum variant B.\n" }
    };

    let mut sources = crate::tests::sources(
        r#"
        //! Mod/file doc.
        /*! Multiline mod/file doc.
         *  :)
         */

        /// Top-level function.
        fn stuff(a, b) {}

        /// Top-level struct.
        /// Second line!
        struct Struct {
            /// Struct field A.
            a,
            /// Struct field B.
            b,
        }

        /**
         * Top-level enum.
         */
        enum Enum {
            /// Enum variant A.
            A,
            /// Enum variant B.
            B {
                /// Enum struct variant B field A.
                a,
                /// Enum struct variant B field B.
                b,
            },
        }

        /// Top-level constant.
        const CONSTANT = 15;

        /// Top-level module.
        mod module {
            //! Also module doc.

            /// Module enum.
            enum Enum {
                /// Enum variant A.
                A,
                /// Enum variant B.
                B,
            }

            /// Module in a module.
            mod Module {
                /// Module enum.
                enum Enum {
                    /// Enum variant A.
                    A,
                    /// Enum variant B.
                    B,
                }
            }
        }
    "#,
    );

    let context = Context::default();

    let _ = prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_visitor(&mut vis)?
        .build()?;

    vis.assert();
    Ok(())
}
