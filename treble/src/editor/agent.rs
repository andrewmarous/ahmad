use std::net::ToSocketAddrs;
use std::time::Duration;
use std::{env, sync::Arc};

use bytes::Bytes;
use crossbeam::channel::Sender;
use nih_plug::{nih_log, nih_error};

use futures::StreamExt;
use iced_futures::stream::try_channel;
use futures::{stream::Stream, SinkExt};
use reqwest::Client;
use url::Url;
use tokio::io::{self, AsyncWriteExt};
use tokio::fs::File;
use serde::{Serialize, Deserialize};

use anyhow::{anyhow, Error};

use std::{
    fs, net::TcpStream, path::PathBuf
};

use crate::{ResponseFiletype, editor::TaskResponse};

#[derive(Debug, Serialize)]
struct GenerationPayload<'a> {
    prompt: &'a str,
    negative_prompt: &'a str,
    #[serde(rename = "output_filetype")]
    filetype: &'a str,
}

fn api_url(endpoint: &str) -> Result<Url, Error> {
    let mut url_base: String = env::var("API_URL").expect("API_URL must be defined");
    nih_log!("{}", url_base);
    url_base.push_str(endpoint);
    match Url::parse(&url_base) {
        Err(e) => Err(Error::new(e)),
        Ok(url) => Ok(url)
    }
}

pub fn check_backend() -> impl Stream<Item= Result<(), Error>> {
    nih_log!("checking backend connection...");
    let port: u16 = 8000;
    let url: String = env::var("API_URL").expect("API_URL must be defined");
    let host = (url, port).to_socket_addrs().unwrap().next().unwrap();
    let timeout = Duration::from_secs(3);
    let port_check = async move {
        TcpStream::connect_timeout(&host, timeout)?;
        Ok(())
    };
    futures::stream::once(port_check)
}

pub fn request_response_stream(
    prompt: String,
    bytestream: Arc<Sender<Bytes>>,
    filetype: ResponseFiletype,
    runtime: Arc<tokio::runtime::Runtime>,
) -> impl Stream<Item= Result<TaskResponse, Error>> {
    try_channel(
        1, move |mut sender| async move {
            sender.send(TaskResponse::Progress(33.0)).await?;

            let client = Client::new();
            let extension = match filetype {
                ResponseFiletype::Wav => "wav",
                ResponseFiletype::Midi => "midi",
            };
            let payload = GenerationPayload {
                prompt: &prompt[..],
                negative_prompt: "Low quality, average quality".into(),
                filetype: extension,
            };
            nih_log!("built request payload.");
            sender.send(TaskResponse::Progress(66.0)).await?;

            let response = runtime.block_on(async {
                client.post(api_url("/generate").expect("Given endpoint is invalid."))
                    .json(&payload)
                    .send()
                    .await
            })?;
            let response = response.error_for_status()?;

            nih_log!("received generate request.");
            sender.send(TaskResponse::Progress(99.0)).await?;

            let mut bytes = response.bytes_stream();
            let Some(Ok(header)) = bytes.next().await else {
                nih_error!("error receiving response: stream is empty");
                // FIX: this error type
                return Err(Error::new(tokio::time::error::Error::shutdown()))
            };

            sender.send(TaskResponse::Bytes(header)).await?;

            // parse header
            while let Some(item)= bytes.next().await {
                let payload = item?;
                bytestream.try_send(
                    payload
                )?;
            }

            // TODO: get this stream's output to plugin's output audio channel somehow
            // let len = bytes.len();
            nih_log!("response bytes received");

            nih_log!("response data successfully received.");
            Ok(())
        }
    )
}

