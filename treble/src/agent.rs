use bytes::Bytes;
use futures::StreamExt;
use futures::{SinkExt, stream::Stream};
use iced_futures::stream::try_channel;
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONNECTION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs::File;
use tokio::io::{self, AsyncWriteExt};
use tokio::time::sleep;
use tracing::{error, info};
use url::Url;

use anyhow::Error;

use std::{
    env, fs,
    net::{TcpStream, ToSocketAddrs},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use crate::TaskResponse;

#[derive(Debug, Serialize)]
struct RunpodBody<'a> {
    input: RunpodRequest<'a>,
}

#[derive(Debug, Serialize)]
struct RunpodRequest<'a> {
    route: &'a str,
    payload: Payload<'a>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Payload<'a> {
    Generate(GenerationPayload<'a>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Response {
    Generate(GenerationResponse),
}

#[derive(Debug, Serialize)]
struct GenerationPayload<'a> {
    prompt: &'a str,
    negative_prompt: &'a str,
    #[serde(rename = "output_filetype")]
    filetype: &'a str,
}

#[derive(Debug, Deserialize)]
struct RunResponse {
    id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    id: String,
    status: String,
    #[serde(rename = "delayTime")]
    delay_time: Option<i64>,
    #[serde(rename = "executionTime")]
    execution_time: Option<i64>,
    output: Option<GenerationResponse>,
}

#[derive(Debug, Deserialize)]
struct GenerationResponse {
    file_data: String,
    file_name: String,
    media_type: String,
    status_code: i16,
}

fn api_url(endpoint: &str) -> Result<Url, Error> {
    let mut url_base: String = String::from(env!("API_URL"));
    url_base.push_str(endpoint);
    match Url::parse(&url_base) {
        Err(e) => Err(Error::new(e)),
        Ok(url) => Ok(url),
    }
}

pub fn check_backend() -> impl Stream<Item = Result<(), Error>> {
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
) -> impl Stream<Item = Result<TaskResponse, Error>> {
    try_channel(1, move |mut sender| async move {
        sender.send(TaskResponse::Progress(33.0)).await?;

        let client = Client::new();
        let headers = {
            let mut res = HeaderMap::new();
            let key = format!("Bearer {}", env!("RUNPOD_API_KEY"));
            res.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            res.append(CONNECTION, HeaderValue::from_static("keep-alive"));
            res.append(
                AUTHORIZATION,
                HeaderValue::from_str(&key).expect("Format of API key is invalid."),
            );
            res
        };
        let Some(filetype) = output_path.extension() else {
            sender
                .send(TaskResponse::String(String::from(
                    "Error: file extension is not valid. Please add a filename that ends with \
                        .midi or .wav",
                )))
                .await?;
            return Ok(());
        };
        let payload = RunpodBody {
            input: RunpodRequest {
                route: "generate",
                payload: Payload::Generate(GenerationPayload {
                    prompt: &prompt[..],
                    negative_prompt: "Low or medium quality",
                    filetype: &filetype.to_str().unwrap(),
                }),
            },
        };
        info!("prompt: {}", &prompt[..]);
        info!("built request payload.");
        sender.send(TaskResponse::Progress(66.0)).await?;

        let resp = client
            .post(api_url("/run").expect("Given endpoint is invalid."))
            .headers(headers.clone())
            .json(&payload)
            .send()
            .await?
            .text()
            .await?;
        let run_response = match serde_json::from_str::<RunResponse>(&resp) {
            Ok(r) => r,
            Err(e) => {
                error!("Error deserializing run response: {}", resp);
                return Err(Error::new(e));
            }
        };

        let response = loop {
            let _ = sleep(Duration::from_secs(10));
            let mut endpoint = String::from("/status/");
            endpoint.push_str(&run_response.id);
            let resp = client
                    .post(api_url(&endpoint[..]).expect("Given endpoint is invalid."))
                    .headers(headers.clone())
                    .send()
                    .await?;
            let resp_text = resp.text().await.unwrap_or(String::new());
            let json = match serde_json::from_str::<StatusResponse>(&resp_text) {
                Ok(r) => r,
                Err(e) => {
                    error!("Error deserializing status response: {}", resp_text);
                    return Err(Error::new(e));
                }
            };
            if json.status == "COMPLETED" { break json; }
        };

        let output: GenerationResponse = response.output
            .expect("Output should not be None outside of loop.");
        let bytes: Bytes = output.file_data.into_bytes().into();

        info!("received generate request.");
        sender.send(TaskResponse::Progress(99.0)).await?;

        info!("building file...");
        let len = bytes.len();
        fs::write(output_path, bytes)?;
        info!("response file successfully built.");
        sender
            .send(TaskResponse::String(String::from(format!("{}", len))))
            .await?;

        Ok(())
    })
}
