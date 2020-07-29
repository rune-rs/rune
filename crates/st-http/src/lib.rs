#[derive(Debug)]
struct Client {
    client: reqwest::Client,
}

#[derive(Debug)]
struct Response {
    response: Option<reqwest::Response>,
}

impl Response {
    async fn text(&mut self) -> st::Result<String> {
        let response = self
            .response
            .take()
            .ok_or_else(|| st::Error::msg("response has already been consumed"))?;

        Ok(response.text().await?)
    }
}

impl Client {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Get the given URL.
    async fn get(&self, url: &str) -> st::Result<Response> {
        let response = reqwest::get(url).await?;

        Ok(Response {
            response: Some(response),
        })
    }
}

st::decl_external!(Client);
st::decl_external!(Response);

/// Install the http library.
pub fn install(functions: &mut st::Functions) -> Result<(), st::RegisterError> {
    let module = functions.module_mut(&["http"])?;
    module.global_fn("client", Client::new)?;

    let module = functions.global_module_mut();
    module.async_instance_fn("get", Client::get)?;
    module.async_instance_fn("text", Response::text)?;
    Ok(())
}
