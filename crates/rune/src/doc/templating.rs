use std::sync::Arc;

use anyhow::Result;
use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext, Renderable, StringOutput,
};
use serde::Serialize;

/// A compiled template.
pub(crate) struct Template {
    handlebars: Arc<Handlebars<'static>>,
    template: handlebars::Template,
}

impl Template {
    /// Render the current template.
    pub(crate) fn render<T>(&self, data: &T) -> Result<String>
    where
        T: Serialize,
    {
        let ctx = Context::wraps(data)?;
        let mut render_context = RenderContext::new(None);
        let mut out = StringOutput::new();
        self.template
            .render(&self.handlebars, &ctx, &mut render_context, &mut out)?;
        Ok(out.into_string()?)
    }
}

/// Templating system.
pub(crate) struct Templating {
    handlebars: Arc<Handlebars<'static>>,
}

impl Templating {
    /// Set up a new templating engine.
    pub(crate) fn new() -> Result<Templating> {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("literal", Box::new(literal));

        Ok(Templating {
            handlebars: Arc::new(handlebars),
        })
    }

    /// Compile the template.
    pub(crate) fn compile(&self, source: &str) -> Result<Template> {
        let template = handlebars::Template::compile(source)?;

        Ok(Template {
            handlebars: self.handlebars.clone(),
            template,
        })
    }
}

fn literal(
    h: &Helper<'_, '_>,
    _: &Handlebars<'_>,
    _: &Context,
    _: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    out.write(param)?;
    Ok(())
}
