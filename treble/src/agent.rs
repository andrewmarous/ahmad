use std::env;

use futures::StreamExt;
use iced_futures::stream::try_channel;
use iced_futures::core::image::Bytes;
use futures::{stream::Stream, SinkExt};
use reqwest::Client;
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

fn api_url(endpoint: &str) -> Url {
    let mut url_base: String = env::var("API_URL").expect("API_URL must be defined");
    url_base.push_str(endpoint);
    Url::parse(&url_base).expect("Given url is not valid.")
}

pub fn check_backend() -> impl Stream<Item= Result<(), Error>> {
    let port: u16 = 8000;
    let url: String = env::var("API_URL").expect("API_URL must be defined");
    let port_check = async move {
        TcpStream::connect((url, port))?;
        Ok(())
    };
    futures::stream::once(port_check)
}

pub fn request_response_stream(prompt: String, output_path: PathBuf) -> impl Stream<Item= Result<String, Error>> {
    try_channel(
        1, move |mut sender| async move {
            sender.send(String::from("33.0")).await?;

            let client = Client::new();
            let payload = GenerationPayload {
                prompt: &prompt[..],
                negative_prompt: "Low quality, average quality".into(),
                filename: "",
            };
            sender.send(String::from("66.0")).await?;

            let response = client
                .post(api_url("generate"))
                .json(&payload)
                .send()
                .await?;
            sender.send(String::from("99.0")).await?;

            let mut file = File::create(&output_path).await?;
            let mut downloaded = 0usize;

            let mut bs = response.bytes_stream();
            while let Some(chunk) = bs.next().await {
                let written = file.write(chunk
                    .unwrap_or(Bytes::new())
                    .as_ref()).await?;
                downloaded += written;
            }
            sender.send(String::from(downloaded.to_string())).await?;

            Ok(())
        }
    )
}

