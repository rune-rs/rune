use crate::alloc::{self, Box, HashMap};
use crate::compile::{IntoComponent, Item, ItemBuf};

/// The contents of a prelude.
#[derive(Default)]
pub struct Prelude {
    /// Prelude imports.
    prelude: HashMap<Box<str>, ItemBuf>,
}

impl Prelude {
    /// Construct a new unit with the default prelude.
    pub(crate) fn with_default_prelude() -> alloc::Result<Self> {
        let mut this = Self::default();

        this.add_prelude("Type", ["any", "Type"])?;
        this.add_prelude("assert_eq", ["test", "assert_eq"])?;
        this.add_prelude("assert_ne", ["test", "assert_ne"])?;
        this.add_prelude("assert", ["test", "assert"])?;
        this.add_prelude("bool", ["bool"])?;
        this.add_prelude("u8", ["u8"])?;
        this.add_prelude("f64", ["f64"])?;
        this.add_prelude("i64", ["i64"])?;
        this.add_prelude("char", ["char"])?;
        this.add_prelude("dbg", ["io", "dbg"])?;
        this.add_prelude("drop", ["mem", "drop"])?;
        this.add_prelude("Err", ["result", "Result", "Err"])?;
        this.add_prelude("file", ["macros", "builtin", "file"])?;
        this.add_prelude("format", ["fmt", "format"])?;
        this.add_prelude("is_readable", ["is_readable"])?;
        this.add_prelude("is_writable", ["is_writable"])?;
        this.add_prelude("line", ["macros", "builtin", "line"])?;
        this.add_prelude("None", ["option", "Option", "None"])?;
        this.add_prelude("Tuple", ["tuple", "Tuple"])?;
        this.add_prelude("Object", ["object", "Object"])?;
        this.add_prelude("Ok", ["result", "Result", "Ok"])?;
        this.add_prelude("Option", ["option", "Option"])?;
        this.add_prelude("panic", ["panic"])?;
        this.add_prelude("print", ["io", "print"])?;
        this.add_prelude("println", ["io", "println"])?;
        this.add_prelude("Result", ["result", "Result"])?;
        this.add_prelude("Some", ["option", "Option", "Some"])?;
        this.add_prelude("String", ["string", "String"])?;
        this.add_prelude("stringify", ["stringify"])?;
        this.add_prelude("Vec", ["vec", "Vec"])?;
        this.add_prelude("Bytes", ["bytes", "Bytes"])?;

        Ok(this)
    }

    /// Access a value from the prelude.
    pub(crate) fn get<'a>(&'a self, name: &str) -> Option<&'a Item> {
        Some(self.prelude.get(name)?)
    }

    /// Define a prelude item.
    fn add_prelude<I>(&mut self, local: &str, path: I) -> alloc::Result<()>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.prelude
            .try_insert(local.try_into()?, ItemBuf::with_crate_item("std", path)?)?;
        Ok(())
    }
}
