use anyhow::Result;
use clap::Parser;

use crate::cli::SharedFlags;
use crate::{Context, Options};

#[derive(Parser, Debug, Clone)]
pub(super) struct Flags {
    #[command(flatten)]
    pub(super) shared: SharedFlags,
}

pub(super) async fn run(context: Context) -> Result<()> {
    let options = Options::default();
    crate::languageserver::run(context, options).await?;
    Ok(())
}
