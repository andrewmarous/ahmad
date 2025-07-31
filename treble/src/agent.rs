use futures::StreamExt;
use iced_futures::stream::try_channel;
use futures::{stream::Stream, SinkExt};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONNECTION, CONTENT_TYPE}, Client
};
use tracing::{info, error};
use url::Url;
use tokio::io::{self, AsyncWriteExt};
use tokio::fs::File;
use serde::{Serialize, Deserialize};
use serde_json::json;
use bytes::Bytes;

use anyhow::Error;

use std::{
    fs, net::{TcpStream, ToSocketAddrs}, path::PathBuf,
    time::Duration, env, sync::Arc

};

use crate::TaskResponse;

#[derive(Debug, Serialize)]
struct RunpodBody<'a> {
    input: RunpodRequest<'a>,
}

#[derive(Debug, Serialize)]
struct RunpodRequest<'a> {
    route: &'a str,
    payload: Payload<'a>
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
                let key = format!("Bearer {}", env!("RUNPOD_API_KEY"));
                res.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                res.append(CONNECTION, HeaderValue::from_static("keep-alive"));
                res.append(AUTHORIZATION, HeaderValue::from_str(&key).expect("Format of API key is invalid."));
                res
            };
            let Some(filetype) = output_path.extension() else {
                sender.send(TaskResponse::String(
                    String::from("Error: file extension is not valid. Please add a filename that ends with \
                        .midi or .wav"))).await?;
                return Ok(());
            };
            let payload = RunpodBody {
                input: RunpodRequest {
                    route: "generate",
                    payload: Payload::Generate(
                        GenerationPayload {
                            prompt: &prompt[..],
                            negative_prompt: "Low or medium quality",
                            filetype: &filetype.to_str().unwrap()
                        }
                    )
            }};
            info!("built request payload.");
            sender.send(TaskResponse::Progress(66.0)).await?;

            let response = client.post(api_url("").expect("Given endpoint is invalid."))
                .headers(headers)
                .json(&payload)
                .send()
                .await?;


            let text = response.text().await?;
            let response = match serde_json::from_str::<GenerationResponse>(&text) {
                Ok(parsed) => parsed,
                Err(e) => {
                    error!("Error deserializing response. Raw response text: {}", &text);
                    return Err(Error::new(e));
                }
            };
            let bytes: Bytes = response.file_data.into_bytes().into();

            info!("received generate request.");
            sender.send(TaskResponse::Progress(99.0)).await?;

            info!("building file...");
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

