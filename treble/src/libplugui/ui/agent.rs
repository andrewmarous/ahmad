use std::env;

use nih_plug::{nih_log, nih_error};

use futures::StreamExt;
use iced_futures::stream::try_channel;
use futures::{stream::Stream, SinkExt};
use reqwest::blocking::Client;
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
            nih_log!("built request payload.");
            sender.send(String::from("66.0")).await?;

            let response = client
                .post(api_url("/generate")?)
                .json(&payload)
                .send()?;

            nih_log!("received generate request.");
            sender.send(String::from("99.0")).await?;

            nih_log!("building file...");
            let bytes = response.bytes()?;
            let len = bytes.len();

            fs::write(output_path, bytes)?;
            nih_log!("response file successfully built.");
            sender.send(String::from(
                format!("{}", len)
            )).await?;

            Ok(())
        }
    )
}

