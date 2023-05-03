use anyhow::Result;

use crate::{Context, Options};

pub(super) async fn run(context: Context) -> Result<()> {
    let options = Options::default();
    crate::languageserver::run(context, options).await?;
    Ok(())
}
