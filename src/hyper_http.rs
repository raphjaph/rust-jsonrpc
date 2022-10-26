use serde;
use std::time::{Duration, Instant};
use std::{error, fmt, io::Error, net, thread};

use crate::client::Transport;
use crate::{Request, Response};

use hyper::client::connect::HttpConnector;
use hyper::Client as HyperClient;
use hyper::{Body, Request as HttpRequest, Response as HttpResponse, Uri};

pub const DEFAULT_PORT: u16 = 8332;

#[derive(Clone, Debug)]
struct HyperTransport {
    uri: Uri,
    timeout: Duration,
    basic_auth: Option<String>,
    client: HyperClient<HttpConnector>,
}

impl HyperTransport {
    pub fn new() -> Self {
        HyperTransport {
            uri: Uri::from_static("127.0.0.1:8332"),
            timeout: Duration::from_secs(15),
            basic_auth: None,
            client: HyperClient::new(),
        }
    }

    fn request<R>(&self, req: impl serde::Serialize) -> Result<R, Error>
    where
        R: for<'a> serde::de::Deserialize<'a>,
    {
        let request_deadline = Instant::now() + self.timeout;
        let body = serde_json::to_vec(&req)?;
        let mut builder = HttpRequest::builder()
            .method("POST")
            .uri(self.uri.path())
            .header("Connection", "Close")
            .header("Content-Type", "application/json")
            .header("Content-Length", body.len().to_string());
        if let Some(ref auth) = self.basic_auth {
            builder = builder.header("Authorization", auth.to_string());
        }

        let request = builder.body(Body::from(body)).unwrap();
        let future = self.client.request(request);
        let response: HttpResponse<R> = future.into();

        Ok(())
    }
}

impl Transport for HyperTransport {
    fn send_request(&self, req: Request) -> Result<Response, crate::Error> {
        Ok(self.request(req)?)
    }

    fn send_batch(&self, reqs: &[Request]) -> Result<Vec<Response>, crate::Error> {
        Ok(self.request(reqs)?)
    }

    fn fmt_target(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "http://{}:{}{}", self.uri.host().unwrap(), self.uri.port().unwrap(), self.uri.path())
    }
}

#[derive(Clone, Debug)]
pub struct Builder {
    transport: HyperTransport,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            transport: HyperTransport::new(),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.transport.timeout = timeout;
        self
    }

    pub fn url(mut self, url: &str) -> Result<Self, Error> {
        self.transport.uri = Uri::from_static(url);
        Ok(self)
    }

    pub fn auth<S: AsRef<str>>(mut self, user: S, pass: Option<S>) -> Self {
        let mut auth = user.as_ref().to_owned();
        auth.push(':');
        if let Some(ref pass) = pass {
            auth.push_str(pass.as_ref());
        }
        self.transport.basic_auth = Some(format!("Basic {}", &base64::encode(auth.as_bytes())));
        self
    }

    pub fn cookie_auth<S: AsRef<str>>(mut self, cookie: S) -> Self {
        self.transport.basic_auth =
            Some(format!("Basic {}", &base64::encode(cookie.as_ref().as_bytes())));
        self
    }

    pub fn build(self) -> HyperTransport {
        self.transport
    }
}

impl crate::Client {
    pub fn hyper_http(
        url: &str,
        user: Option<String>,
        pass: Option<String>,
    ) -> Result<crate::Client, Error> {
        let mut builder = Builder::new().url(url)?;
        if let Some(user) = user {
            builder = builder.auth(user, pass);
        }
        Ok(crate::Client::with_transport(builder.build()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //    #[test]
    //    fn construct() {
    //        let tp = Builder::new()
    //            .timeout(Duration::from_millis(100))
    //            .url("localhost:22")
    //            .unwrap()
    //            .auth("user", None)
    //            .build();
    //        let _ = Client::with_transport(tp);
    //
    //        let _ = Client::simple_http("localhost:22", None, None).unwrap();
    //    }
}
