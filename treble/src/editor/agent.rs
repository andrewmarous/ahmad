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

use crate::ResponseFiletype;

#[derive(Debug, Serialize)]
struct GenerationPayload<'a> {
    prompt: &'a str,
    negative_prompt: &'a str,
    #[serde(rename = "client_output_path")]
    filetype: &'a str,
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
    nih_log!("checking backend connection...");
    let port: u16 = 8000;
    let url: String = env::var("API_URL").expect("API_URL must be defined");
    let port_check = async move {
        TcpStream::connect((url, port))?;
        Ok(())
    };
    futures::stream::once(port_check)
}

pub fn request_response_stream(
    prompt: String,
    bytestream: Arc<Sender<Result<Bytes, Error>>>,
    filetype: ResponseFiletype,
) -> impl Stream<Item= Result<String, Error>> {
    try_channel(
        1, move |mut sender| async move {
            sender.send(String::from("33.0")).await?;

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
            sender.send(String::from("66.0")).await?;

            let response = client
                .post(api_url("/generate").expect("Given endpoint is invalid."))
                .json(&payload)
                .send()
                .await?;

            nih_log!("received generate request.");
            sender.send(String::from("99.0")).await?;

            let mut bytes = response.bytes_stream();
            while let Some(item)= bytes.next().await {
                bytestream.try_send(
                    item.map_err(|err| {
                        anyhow!(err)
                    })
                )?;
            }

            // TODO: get this stream's output to plugin's output audio channel somehow
            // let len = bytes.len();
            nih_log!("response bytes received");

            nih_log!("response data successfully received.");
            sender.send(String::from(
                format!("{}", -1)
            )).await?;

            Ok(())
        }
    )
}

