use futures::StreamExt;
use iced_futures::stream::try_channel;
use futures::{stream::Stream, SinkExt};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderName, HeaderValue, CONNECTION},
};
use tracing::info;
use url::Url;
use tokio::io::{self, AsyncWriteExt};
use tokio::fs::File;
use serde::{Serialize, Deserialize};

use anyhow::Error;

use std::{
    fs, net::{TcpStream, ToSocketAddrs}, path::PathBuf,
    time::Duration, env, sync::Arc

};

use crate::TaskResponse;

#[derive(Debug, Serialize)]
struct GenerationPayload<'a> {
    prompt: &'a str,
    negative_prompt: &'a str,
    #[serde(rename = "output_filetype")]
    filetype: &'a str,
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

pub fn check_backend() -> impl Stream<Item= Result<(), Error>> {
    info!("checking backend connection...");
    futures::stream::once(async move {
        let url = api_url("")?;
        let socket_addr = (url.host_str().unwrap(), 8000)
            .to_socket_addrs()
            .expect("DNS lookup failed")
            .next()
            .expect("no socket addresses returned");

        let timeout = Duration::from_secs(3);
        TcpStream::connect_timeout(&socket_addr, timeout)?;
        Ok(())
    })
}

pub fn request_response_stream(
    prompt: String,
    output_path: PathBuf,
) -> impl Stream<Item= Result<TaskResponse, Error>> {
    try_channel(
        1, move |mut sender| async move {
            sender.send(TaskResponse::Progress(33.0)).await?;

            let client = Client::new();
            let headers = {
                let mut res = HeaderMap::new();
                let k = "keep-alive";
                res.append(CONNECTION, HeaderValue::from_static(k));
                res.append(HeaderName::from_static(k), HeaderValue::from_static("timeout=300, max=50"));
                res
            };
            let Some(filetype) = output_path.extension() else {
                sender.send(TaskResponse::String(
                    String::from("Error: file extension is not valid. Please add a filename that ends with \
                        .midi or .wav"))).await?;
                return Ok(());
            };
            let payload = GenerationPayload {
                prompt: &prompt[..],
                negative_prompt: "Low quality, average quality".into(),
                filetype: filetype.to_str().unwrap()
            };
            info!("built request payload.");
            sender.send(TaskResponse::Progress(66.0)).await?;

            let response = client.post(api_url("/generate").expect("Given endpoint is invalid."))
                .headers(headers)
                .json(&payload)
                .send()
                .await?;

            info!("received generate request.");
            sender.send(TaskResponse::Progress(99.0)).await?;

            info!("building file...");
            let bytes = response.bytes().await?;
            let len = bytes.len();

            fs::write(output_path, bytes)?;
            info!("response file successfully built.");
            sender.send(TaskResponse::String(String::from(
                format!("{}", len)
            ))).await?;

            Ok(())
        }
    )
}

