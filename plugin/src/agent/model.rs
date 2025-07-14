use std::{path::Path, sync::Arc};

use axum::{
    Router,
    extract::{FromRef, State, Json},
    response::{
        Sse,
        sse::{Event, KeepAlive}
    },
    routing::post
};
use serde::Deserialize;
use futures::Stream;
use ort::{
    session::{RunOptions, Session, builder::GraphOptimizationLevel},
    value::TensorRef
};
use tokenizers::Tokenizer;
use tokio::{net::TcpListener, sync::Mutex};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};


// Max number of generated tokens
const GEN_TOKENS: usize = 1;
// Sample from the k most likely next tokens at each step
const TOP_K: usize = 20;

pub async fn create() -> anyhow::Result<()> {
    // Initialize tracing to recieve debug messages from "ort"
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,ort=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load model
    let mut session = Session::builder()?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .with_intra_threads(4)?
    .commit_from_file("")?;

    // Load the tokenizer and encode the prompt into a sequence of tokens
    let tokenizer = Tokenizer::from_file(
        Path::new(env!("MODEL_STORE_ROOT"))
            .parent()
            .unwrap()
            .join("tokenizer.json")
    ).unwrap();

    let app_state = AppState {
        session: Arc::new(Mutex::new(session)),
        tokenizer: Arc::new(tokenizer)
    };

    let app = Router::new().route("/generate", post(generate)).with_state(app_state).into_make_service();
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    tracing::info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    session: Arc<Mutex<Session>>,
    tokenizer: Arc<Tokenizer>
}

fn generate_stream(
    tokenizer: Arc<Tokenizer>,
    session: Arc<Mutex<Session>>,
    mut tokens: Vec<i64>,
    gen_tokens: usize
) -> impl Stream<Item = ort::Result<Event>> + Send {
    async_stream_lite::try_async_stream(|yielder| async move {
        for _ in 0..gen_tokens {
            let input = TensorRef::from_array_view((vec![1, 1, tokens.len() as i64], tokens.as_slice()))?;
            let probabilities = {
                let mut session = session.lock().await;
                let options = RunOptions::new()?;
                let outputs = session.run_async(ort::inputs![input], &options)?.await?;
                let (dim, probabilities) = outputs["output1"].try_extract_tensor()?;

                // collect logits
                let (seq_len, vocab_size) = (dim[2] as usize, dim[3] as usize);
                let mut probabilities: Vec<(usize, f32)> = probabilities[(seq_len - 1) * vocab_size..].iter().copied().enumerate().collect();
                probabilities.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Less));
                probabilities
            };

            let token = probabilities[0].0 as i64;
            tokens.push(token);

            let token_str = tokenizer.decode(&[token as _], true).unwrap();
            yielder.r#yield(Event::default().data(token_str)).await;
        }

        Ok(())
    })
}

#[derive(Deserialize)]
struct PromptRequest {
    prompt: String,
}

impl FromRef<AppState> for Arc<Mutex<Session>> {
    fn from_ref(input: &AppState) -> Self {
        Arc::clone(&input.session)
    }
}

impl FromRef<AppState> for Arc<Tokenizer> {
    fn from_ref(input: &AppState) -> Self {
        Arc::clone(&input.tokenizer)
    }
}

async fn generate(
    State(session): State<Arc<Mutex<Session>>>,
    State(tokenizer): State<Arc<Tokenizer>>,
    Json(body): Json<PromptRequest>
)-> Sse<impl Stream<Item = ort::Result<Event>>> {
    let encoding = tokenizer
        .encode(body.prompt.clone(), true)
        .map_err(|e| {
            tracing::error!("tokenizer error: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        });

    let tokens: Vec<i64> = encoding
        .unwrap()
        .get_ids()
        .iter()
        .map(|&id| id as i64)
        .collect();
    Sse::new(generate_stream(tokenizer, session, tokens, GEN_TOKENS)).keep_alive(KeepAlive::new())
}

