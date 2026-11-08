//! Adapter from the LSP semantic-fact boundary to the neutral
//! [`opencode_retrieval::ExactFactProvider`] contract.
//!
//! ScopeMux and other consumers depend on the neutral trait, never on this crate or
//! `lsp_types`. The runtime injects this adapter only when LSP is configured.

use std::sync::Arc;

use async_trait::async_trait;
use opencode_retrieval::{
    ExactFact, ExactFactKind, ExactFactLocation, ExactFactOutcome, ExactFactProvider,
    ExactFactRequest, ExactFactUnavailable,
};

use crate::{
    FactUnavailable, LspClientRegistry, SemanticFactKind, SemanticFactOutcome, SemanticFactRequest,
};

/// Exact-fact source backed by the configured LSP registry.
#[derive(Clone)]
pub struct LspExactFactProvider {
    registry: Arc<LspClientRegistry>,
}

impl LspExactFactProvider {
    pub fn new(registry: Arc<LspClientRegistry>) -> Self {
        Self { registry }
    }
}

fn kind_to_lsp(kind: ExactFactKind) -> SemanticFactKind {
    match kind {
        ExactFactKind::Definition => SemanticFactKind::Definition,
        ExactFactKind::References => SemanticFactKind::References,
        ExactFactKind::TypeDefinition => SemanticFactKind::TypeDefinition,
        ExactFactKind::Implementation => SemanticFactKind::Implementation,
    }
}

fn reason_to_neutral(reason: FactUnavailable) -> ExactFactUnavailable {
    match reason {
        FactUnavailable::NoServer => ExactFactUnavailable::NoServer,
        FactUnavailable::Unsupported => ExactFactUnavailable::Unsupported,
        FactUnavailable::Timeout => ExactFactUnavailable::Timeout,
        FactUnavailable::Failure => ExactFactUnavailable::Failure,
    }
}

#[async_trait]
impl ExactFactProvider for LspExactFactProvider {
    async fn exact_facts(&self, request: ExactFactRequest) -> ExactFactOutcome {
        let response = self
            .registry
            .semantic_facts(SemanticFactRequest::new(
                request.path,
                request.line,
                request.character,
                kind_to_lsp(request.kind),
            ))
            .await;

        let cached = response.cached;
        match response.outcome {
            SemanticFactOutcome::Exact(fact) => ExactFactOutcome::Exact(ExactFact {
                kind: request.kind,
                locations: fact
                    .locations
                    .into_iter()
                    .map(|location| ExactFactLocation {
                        file: location.file.display().to_string(),
                        start_line: location.start.line,
                        start_character: location.start.character,
                        end_line: location.end.line,
                        end_character: location.end.character,
                    })
                    .collect(),
                server_id: fact.server_id,
                document_version: fact.document_version,
                file_hash: fact.file_hash,
                cached,
            }),
            SemanticFactOutcome::Unavailable { reason, .. } => ExactFactOutcome::Unavailable {
                reason: reason_to_neutral(reason),
            },
        }
    }
}
