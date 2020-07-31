use st::packages::bytes::Bytes;

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

#[derive(Debug)]
pub struct RequestBuilder {
    request: reqwest::RequestBuilder,
}

impl RequestBuilder {
    /// Send the request being built.
    async fn send(self) -> st::Result<Response> {
        let response = self.request.send().await?;
        Ok(Response { response })
    }

    async fn body_bytes(self, bytes: Bytes) -> st::Result<Self> {
        let bytes = bytes.into_inner();

        Ok(Self {
            request: self.request.body(bytes),
        })
    }
}

impl Client {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Construct a builder to GET the given URL.
    async fn get(&self, url: &str) -> st::Result<RequestBuilder> {
        let request = self.client.get(url);
        Ok(RequestBuilder { request })
    }

    /// Construct a builder to POST to the given URL.
    async fn post(&self, url: &str) -> st::Result<RequestBuilder> {
        let request = self.client.post(url);
        Ok(RequestBuilder { request })
    }
}

/// Shorthand for generating a get request.
async fn get(url: &str) -> st::Result<Response> {
    Ok(Response {
        response: reqwest::get(url).await?,
    })
}

st::decl_external!(Client);
st::decl_external!(Response);
st::decl_external!(RequestBuilder);

/// Construct the http library.
pub fn module() -> Result<st::Module, st::RegisterError> {
    let mut module = st::Module::new(&["http"]);
    module.global_fn("client", Client::new)?;
    module.async_fn("get", get)?;

    module.async_instance_fn("get", Client::get)?;
    module.async_instance_fn("post", Client::post)?;

    module.async_instance_fn("text", Response::text)?;

    module.async_instance_fn("send", RequestBuilder::send)?;
    module.async_instance_fn("body_bytes", RequestBuilder::body_bytes)?;
    Ok(module)
}
