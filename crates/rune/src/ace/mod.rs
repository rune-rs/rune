mod autocomplete;
pub(crate) use self::autocomplete::build as build_autocomplete;

use anyhow::{anyhow, Context as _, Result};

use crate::alloc::borrow::Cow;
use crate::doc::Artifacts;

mod embed {
    #[cfg(debug_assertions)]
    use rust_alloc::boxed::Box;
    #[cfg(debug_assertions)]
    use rust_alloc::string::String;

    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "src/ace/static"]
    pub(super) struct Assets;
}

pub(crate) fn theme(artifacts: &mut Artifacts) -> Result<()> {
    for name in ["rune-mode.js", "rune-highlight-rules.js"] {
        artifacts.asset(false, name, || {
            let file = embed::Assets::get(name).with_context(|| anyhow!("missing {name}"))?;
            Ok(Cow::try_from(file.data)?)
        })?;
    }

    Ok(())
}
