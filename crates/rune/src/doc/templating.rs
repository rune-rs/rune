use crate::alloc::borrow::Cow;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap, String};
use crate::std::sync::Mutex;
use ::rust_alloc::sync::Arc;

use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext, Renderable,
    StringOutput,
};
use serde::Serialize;

use crate::support::Result;

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
        let cx = Context::wraps(data)?;
        let mut render_context = RenderContext::new(None);
        let mut out = StringOutput::new();
        self.template
            .render(&self.handlebars, &cx, &mut render_context, &mut out)?;
        Ok(out.into_string()?.try_into()?)
    }
}

#[derive(Default, Clone)]
pub(crate) struct Paths {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl Paths {
    /// Insert a path redirect.
    pub(crate) fn insert(&self, from: &str, to: &str) -> alloc::Result<()> {
        self.inner
            .lock()
            .unwrap()
            .try_insert(from.try_to_owned()?, to.try_to_owned()?)?;
        Ok(())
    }
}

/// Templating system.
pub(crate) struct Templating {
    handlebars: Arc<Handlebars<'static>>,
}

impl Templating {
    /// Set up a new templating engine.
    pub(crate) fn new<'a, I>(partials: I, paths: Paths) -> Result<Templating>
    where
        I: IntoIterator<Item = (&'a str, Cow<'a, str>)>,
    {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("literal", ::rust_alloc::boxed::Box::new(literal));
        handlebars.register_helper("path", ::rust_alloc::boxed::Box::new(path(paths)));

        for (name, source) in partials {
            handlebars.register_partial(name, source.as_ref())?;
        }

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

fn path(paths: Paths) -> impl HelperDef + Send + Sync + 'static {
    move |h: &Helper<'_, '_>,
          _: &Handlebars<'_>,
          _: &Context,
          _: &mut RenderContext<'_, '_>,
          out: &mut dyn Output|
          -> HelperResult {
        let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
        let inner = paths.inner.lock().unwrap();
        let path = inner.get(param).map(String::as_str).unwrap_or(param);
        out.write(path)?;
        Ok(())
    }
}
