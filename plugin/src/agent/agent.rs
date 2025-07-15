use iced_futures::stream::try_channel;
use futures::stream::Stream;

use anyhow::Error;

use std::{
    path::PathBuf,
    net::TcpStream,
    io::{Write, Read}, fs
};


use crate::agent::model;

pub fn initialize() -> impl Stream<Item= Result<(), Error>> {
    try_channel(
        1, move |mut sender| async move {
            model::create().await?;
            sender.try_send(())?;
            Ok(())
        }
    )
}

pub fn request_response_stream(prompt: String, output_path: PathBuf) -> impl Stream<Item= Result<String, Error>> {
    try_channel(
        1, move |mut sender| async move {
            let mut stream = TcpStream::connect("127.0.0.1:8000")?;
            sender.try_send(String::from("25.0"))?;

            stream
                .write_all(prompt.as_bytes())
                .and_then(|_| stream.write_all(b"\n"))?;
            sender.try_send(String::from("50.0"))?;

            let mut buf = Vec::new();
            stream
                .read_to_end(&mut buf)?;
            let response = String::from_utf8(buf)?;
            sender.try_send(String::from("75.0"))?;

            fs::write(output_path, &response)?;
            sender.try_send(response)?;

            Ok(())
        }
    )
}
