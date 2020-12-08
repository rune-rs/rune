use crate::compiling::v1::assemble::prelude::*;
use crate::query::BuiltInFormat;
use runestick::format;

/// Compile a literal template string.
impl Assemble for BuiltInFormat {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span;
        log::trace!("BuiltInFormat => {:?}", c.source.source(span));

        let fill = if let Some((_, fill)) = &self.fill {
            *fill
        } else {
            ' '
        };

        let align = if let Some((_, align)) = &self.align {
            *align
        } else {
            format::Alignment::default()
        };

        let flags = if let Some((_, flags)) = &self.flags {
            *flags
        } else {
            format::Flags::default()
        };

        let width = if let Some((_, width)) = &self.width {
            *width
        } else {
            None
        };

        let precision = if let Some((_, precision)) = &self.precision {
            *precision
        } else {
            None
        };

        let format_type = if let Some((_, format_type)) = &self.format_type {
            *format_type
        } else {
            format::Type::default()
        };

        let spec = format::FormatSpec::new(flags, fill, align, width, precision, format_type);

        self.value.assemble(c, Needs::Value)?.apply(c)?;
        c.asm.push(Inst::Format { spec }, span);

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(Asm::top(span))
    }
}
