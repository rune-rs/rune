use std::collections::BTreeMap;
use rune::compile::{CompileVisitor, MetaRef};
use rune::{Context, Diagnostics, SourceId};
use rune::ast::Span;
use rune::termcolor::{ColorChoice, StandardStream};
use rune_tests::{sources};

struct DocVisitor {
    expected: BTreeMap<&'static str, Vec<&'static str>>,
    collected: BTreeMap<String, Vec<String>>
}

impl CompileVisitor for DocVisitor {
    fn visit_doc_comment(&mut self, _: SourceId, meta: MetaRef<'_>, _: Span, doc: &str) {
        self.collected.entry(meta.item.to_string()).or_default().push(doc.to_string());
    }

    fn visit_file_doc_comment(&mut self, _: SourceId, _:Span, doc: &str) {
        self.collected.entry("".to_string()).or_default().push(doc.to_string());
    }
}

impl DocVisitor {
    fn assert(&self) {
        for (&item, expected) in &self.expected {
            let against = if let Some(vec) = self.collected.get(item) {
                vec
            } else {
                panic!("missing documentation for item {item:?}");
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
            let vec = self.collected.keys().filter(|it| !self.expected.contains_key(it.as_str())).collect::<Vec<_>>();
            panic!("encountered more documented items than expected: {vec:?}");
        }
    }
}

macro_rules! expect_docs {
    ($($typename:literal => { $($docstr:literal)* })+) => {
        {
            let mut expected = BTreeMap::new();

            $(
            #[allow(unused_mut)]
            let mut vec = Vec::new();
            $(vec.push($docstr);
            )*

            expected.insert($typename, vec);
            )+

            DocVisitor {
                expected,
                collected: BTreeMap::new()
            }
        }
    };
}

#[test]
fn harvest_docs() {
    let mut diagnostics = Diagnostics::new();
    let mut vis = expect_docs! {
        "" => {
            " Mod/file doc.\n"
            " Multiline mod/file doc.\n         *  :)\n         "
        }
        "stuff" => { " Top-level function.\n" }
        "Struct" => {
            " Top-level struct.\n"
            " Second line!\n"
        }
        "Enum" => { "\n         * Top-level enum.\n         " }
        "Enum::A" => { " Enum variant A.\n" }
        "Enum::B" => { " Enum variant B.\n" }
        "constant" => { " Top-level constant.\n" }

        "Module" => {
            " Top-level module.\n"
            " Also module doc.\n"
        }
        "Module::Enum" => { " Module enum.\n" }
        "Module::Enum::A" => { " Enum variant A.\n" }
        "Module::Enum::B" => { " Enum variant B.\n" }

        "Module::Module" => { " Module in a module.\n" }
        "Module::Module::Enum" => { " Module enum.\n" }
        "Module::Module::Enum::A" => { " Enum variant A.\n" }
        "Module::Module::Enum::B" => { " Enum variant B.\n" }
    };

    let mut sources = sources(r#"
        //! Mod/file doc.
        /*! Multiline mod/file doc.
         *  :)
         */

        /// Top-level function.
        fn stuff(a, b) {}

        /// Top-level struct.
        /// Second line!
        struct Struct {
            // note: doc comments on struct fields will cause a compile error
            // currently unsupported
            a,
            b,
        }

        /**
         * Top-level enum.
         */
        enum Enum {
            /// Enum variant A.
            A,
            /// Enum variant B.
            B,
        }

        /// Top-level constant.
        const constant = 15;

        /// Top-level module.
        mod Module {
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
    "#);

    let context = Context::default();
    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_visitor(&mut vis as &mut dyn CompileVisitor)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources).unwrap();
    }

    result.unwrap();
    vis.assert();
}