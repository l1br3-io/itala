# Ollama Integration

Integrate the local Ollama LLM runner into Itala's `crates/itala-ai` as one
adapter behind the `Provider` trait.

## Required reading

- `rules/rust/error-handling.md`, `rules/rust/async.md`
- `agents/ai-orchestrator.yaml` for the surrounding orchestrator design

## Setup (user side)

```bash
# Install Ollama from https://ollama.ai
ollama serve                  # background
ollama pull llama3.1          # or any other model
```

Default endpoint: `http://localhost:11434`. Itala does not start Ollama itself.

## Adapter implementation

```rust
// crates/itala-ai/src/adapters/ollama.rs
use crate::{Provider, ProviderMeta, Result};
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use tokio_util::sync::CancellationToken;

pub struct OllamaAdapter {
    client: Ollama,
    default_model: String,
}

impl OllamaAdapter {
    #[must_use]
    pub fn new(host: &str, port: u16, default_model: impl Into<String>) -> Self {
        Self {
            client: Ollama::new(host.to_string(), port),
            default_model: default_model.into(),
        }
    }
}

impl Provider for OllamaAdapter {
    fn metadata(&self) -> ProviderMeta {
        ProviderMeta {
            id: "ollama".into(),
            kind_supported: &[ModelKind::Chat, ModelKind::Embed],
            local: true,
        }
    }

    async fn complete(&self, req: CompleteRequest, ct: CancellationToken) -> Result<CompleteResponse> {
        let model = req.model.unwrap_or_else(|| self.default_model.clone());
        let gen = GenerationRequest::new(model, req.prompt);

        tokio::select! {
            _ = ct.cancelled() => Err(crate::Error::Cancelled),
            res = self.client.generate(gen) => {
                let r = res.map_err(crate::Error::Provider)?;
                Ok(CompleteResponse { text: r.response, .. })
            }
        }
    }

    fn stream(&self, req: CompleteRequest, ct: CancellationToken)
        -> impl futures::Stream<Item = Result<CompleteChunk>>
    {
        // Use `client.generate_stream`; map each chunk through error type.
        unimplemented!()
    }

    async fn embed(&self, req: EmbedRequest, ct: CancellationToken) -> Result<EmbedResponse> {
        // ollama-rs has embeddings via `client.generate_embeddings`.
        unimplemented!()
    }
}
```

## Health check

```rust
pub async fn is_running(&self) -> bool {
    self.client.list_local_models().await.is_ok()
}
```

The orchestrator calls this when picking a chain — Ollama as the first step is skipped if it's not running.

## Models commonly used

- `llama3.1` — general purpose, good default.
- `qwen2.5-coder` — code-focused.
- `nomic-embed-text` — local embeddings.

The adapter does not enforce a model list. The user configures their preferred chain in settings; the adapter only validates that the model is available before issuing the request.

## Streaming

Ollama supports streaming. Map the stream through `tokio_stream` adapters so the orchestrator's streaming consumers see a uniform `Stream<Item = Result<CompleteChunk>>`.

## Privacy

Ollama runs locally — no redaction layer is applied (per `agents/ai-orchestrator.yaml`). Cloud providers do go through redaction.

## Testing

- Unit: mock `ollama_rs::Ollama` via a thin trait wrapper if needed; otherwise rely on integration tests.
- Integration: `crates/itala-ai/tests/ollama_smoke.rs` is gated behind `cfg(feature = "live-ollama")` so CI doesn't require Ollama running. Local devs can opt in.

## Anti-patterns

- Spawning Ollama from Rust. Itala only consumes a running instance.
- Hardcoding model names. The user picks via settings.
- Ignoring `ct.cancelled()`. All long-running calls are cancellable.
