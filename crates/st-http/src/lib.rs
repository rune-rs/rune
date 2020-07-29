#[derive(Debug)]
struct Client {
    client: reqwest::Client,
}

#[derive(Debug)]
pub struct Response {
    response: reqwest::Response,
}

impl Response {
    async fn text(self) -> st::Result<String> {
        let text = self.response.text().await?;
        Ok(text)
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
        Ok(Response { response })
    }
}

st::decl_external!(Client);
st::decl_external!(Response);

/// Construct the http library.
pub fn module() -> Result<st::Module, st::RegisterError> {
    let mut module = st::Module::new(&["http"]);
    module.global_fn("client", Client::new)?;
    module.async_instance_fn("get", Client::get)?;
    module.async_instance_fn("text", Response::text)?;
    Ok(module)
}
