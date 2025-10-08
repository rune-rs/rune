use anyhow::Result;

use crate::{
    languageserver::{Input, Output},
    Context, Options,
};

pub(super) async fn run(context: Context) -> Result<()> {
    let options = Options::from_default_env()?;
    crate::languageserver::run(
        context,
        options,
        (Input::from_stdin()?, Output::from_stdout()?),
    )
    .await?;
    Ok(())
}
