use std::env;

use futures::{
    StreamExt, SinkExt, stream::Stream,
    executor::{block_on, block_on_stream},
    channel::mpsc,
};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONNECTION};
use tracing::info;
use url::Url;
use tokio::io::{self, AsyncWriteExt};
use tokio::fs::File;
use serde::{Serialize, Deserialize};

use anyhow::Error;

use std::{
    fs, net::TcpStream, path::PathBuf
};

#[derive(Debug, Serialize)]
struct GenerationPayload<'a> {
    prompt: &'a str,
    negative_prompt: &'a str,
    #[serde(rename = "client_output_path")]
    filename: &'a str,
}

#[derive(Debug, Deserialize)]
struct GenerationResponse {
}

fn api_url(endpoint: &str) -> Result<Url, Error> {
    let mut url_base: String = env::var("API_URL").expect("API_URL must be defined");
    url_base.push_str(endpoint);
    match Url::parse(&url_base) {
        Err(e) => Err(Error::new(e)),
        Ok(url) => Ok(url)
    }
}

pub fn check_backend() -> Result<(), Error> {
    info!("checking backend connection...");
    let port: u16 = 8000;
    let url: String = env::var("API_URL").expect("API_URL must be defined");
    TcpStream::connect((url, port))?;
    Ok(())
}

pub fn request_response_iterator(prompt: String) -> impl Iterator<Item= Result<String, Error>> {
    let (mut tx, rx) = mpsc::channel(2);
    std::thread::spawn(move || -> Result<(), Error> {
        tx.try_send(Ok(String::from("33.0")))?;

        let client = Client::new();
        let headers = {
            let mut res = HeaderMap::new();
            let k = "keep-alive";
            res.append(CONNECTION, HeaderValue::from_static(k));
            res.append(HeaderName::from_static(k), HeaderValue::from_static("timeout=300, max=50"));
            res
        };
        let payload = GenerationPayload {
            prompt: &prompt[..],
            negative_prompt: "Low quality, average quality".into(),
            filename: "",
        };
        info!("built request payload.");
        tx.try_send(Ok("66.0".into()))?;

        let response = client
            .post(api_url("generate")?)
            .headers(headers)
            .json(&payload)
            .send()?;

        info!("received generate request.");
        tx.try_send(Ok("99.0".into()))?;

        // TODO: how to pass bytes to plugin?

        // FIX: response bytes should go to plugin, not via request/ui
        let bytes = String::new();
        tx.try_send(Ok(bytes))?;

        Ok(())
    });

    block_on_stream(rx)
}


