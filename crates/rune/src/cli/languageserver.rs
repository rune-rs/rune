use anyhow::Result;

use crate::{Context, Options};

pub(super) async fn run(context: Context) -> Result<()> {
    let options = Options::from_default_env()?;
    crate::languageserver::run(context, options).await?;
    Ok(())
}
