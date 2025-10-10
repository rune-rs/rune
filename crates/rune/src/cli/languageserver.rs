use anyhow::Result;

use crate::languageserver;
use crate::{Context, Options};

pub(super) async fn run(context: Context) -> Result<()> {
    let options = Options::from_default_env()?;

    let ls = languageserver::builder()
        .with_context(context)
        .with_options(options)
        .with_stdio()
        .build()?;

    ls.run().await?;
    Ok(())
}
