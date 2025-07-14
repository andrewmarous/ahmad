use iced_futures::stream::try_channel;
use futures::{stream::Stream, executor::block_on};
use anyhow;

use std::{
    thread,
    path::PathBuf,
    time::Duration,
    net::TcpStream,
    io::{Write, Read}, fs
};


use crate::agent::model;

pub fn initialize() -> anyhow::Result<()> {
    block_on(model::create())
}

pub fn request_response_stream(prompt: String, output_path: PathBuf) -> impl Stream<Item= Result<String, ()>> {
    try_channel(
        1, move |mut sender| async move {
            let mut stream = TcpStream::connect("127.0.0.1:8000")
                .map_err(|_| ())?;

            stream
                .write_all(prompt.as_bytes())
                .and_then(|_| stream.write_all(b"\n"))
                .map_err(|_| ())?;

            let mut buf = Vec::new();
            stream
                .read_to_end(&mut buf)
                .map_err(|_| ())?;
            let response = String::from_utf8(buf)
                .map_err(|_| ())?;

            fs::write(output_path, &response)
            .map_err(|_| ())?;

            sender.try_send(response).map_err(|_| ())?;

            Ok(())
        }
    )
}
