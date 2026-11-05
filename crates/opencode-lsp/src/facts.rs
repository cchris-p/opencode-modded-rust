//! Capability-negotiated semantic-fact interface.
//!
//! This is the narrow contract consumers (notably ScopeMux) use instead of importing
//! `lsp_types` or issuing LSP requests directly. Every fact is `exact`, carries server
//! provenance and the document version/file hash it was resolved against, and is cached
//! by content hash. Unavailable capability, missing server, timeout, and failure are
//! explicit outcomes, never errors that fail the caller.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Duration;

use lsp_types::{Location, Position, Range, TextDocumentIdentifier, TextDocumentPositionParams};
use serde::{Deserialize, Serialize};

use crate::{uri_to_path, LspClient, LspClientRegistry, LspError};

/// Semantic questions the fact boundary can answer. LSP owns all of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticFactKind {
    Definition,
    References,
    TypeDefinition,
    Implementation,
    /// Supertypes via the type hierarchy (`typeHierarchy/supertypes`).
    Supertypes,
    IncomingCalls,
    OutgoingCalls,
}

impl SemanticFactKind {
    /// `ServerCapabilities` key this kind needs. The server is never asked for a
    /// capability it did not advertise.
    pub fn capability_key(self) -> &'static str {
        match self {
            SemanticFactKind::Definition => "definitionProvider",
            SemanticFactKind::References => "referencesProvider",
            SemanticFactKind::TypeDefinition => "typeDefinitionProvider",
            SemanticFactKind::Implementation => "implementationProvider",
            SemanticFactKind::Supertypes => "typeHierarchyProvider",
            SemanticFactKind::IncomingCalls | SemanticFactKind::OutgoingCalls => {
                "callHierarchyProvider"
            }
        }
    }
}

/// Exact facts are the only confidence this boundary produces. Heuristic resolution is
/// ScopeMux's job, not LSP's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FactConfidence {
    Exact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactPosition {
    /// 0-based line, as defined by LSP.
    pub line: u32,
    /// 0-based UTF-16 character offset, as defined by LSP.
    pub character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactLocation {
    pub file: PathBuf,
    pub start: FactPosition,
    pub end: FactPosition,
}

/// One exact semantic fact. Locations are neutral paths/positions, not `lsp_types`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticFact {
    pub kind: SemanticFactKind,
    pub locations: Vec<FactLocation>,
    pub confidence: FactConfidence,
    /// Server id that produced the fact.
    pub server_id: String,
    /// Workspace root the server was started for.
    pub workspace_root: PathBuf,
    /// Tracked document version at request time, when the document was opened.
    pub document_version: Option<i32>,
    /// Content hash of the file the fact was resolved against.
    pub file_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FactUnavailable {
    /// No `lsp` config covers this file.
    NoServer,
    /// The server did not advertise the required capability.
    Unsupported,
    /// The request exceeded the bounded timeout.
    Timeout,
    /// The server failed, exited, or returned an invalid response.
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticFactOutcome {
    Exact(SemanticFact),
    Unavailable {
        reason: FactUnavailable,
        detail: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticFactResponse {
    pub outcome: SemanticFactOutcome,
    /// True when the response was served from the in-process fact cache.
    pub cached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticFactRequest {
    pub path: PathBuf,
    /// 0-based line.
    pub line: u32,
    /// 0-based character offset.
    pub character: u32,
    pub kind: SemanticFactKind,
}

impl SemanticFactRequest {
    pub fn new(
        path: impl Into<PathBuf>,
        line: u32,
        character: u32,
        kind: SemanticFactKind,
    ) -> Self {
        Self {
            path: path.into(),
            line,
            character,
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FactCacheKey {
    pub(crate) server_id: String,
    pub(crate) path: PathBuf,
    pub(crate) line: u32,
    pub(crate) character: u32,
    pub(crate) kind: SemanticFactKind,
    pub(crate) file_hash: u64,
}

fn canonicalize_or(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn hash_file(path: &Path) -> std::io::Result<u64> {
    let bytes = std::fs::read(path)?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(hasher.finish())
}

fn range_to_locations(range: &Range) -> (FactPosition, FactPosition) {
    (
        FactPosition {
            line: range.start.line,
            character: range.start.character,
        },
        FactPosition {
            line: range.end.line,
            character: range.end.character,
        },
    )
}

fn location_to_fact(location: &Location) -> FactLocation {
    let (start, end) = range_to_locations(&location.range);
    FactLocation {
        file: uri_to_path(&location.uri),
        start,
        end,
    }
}

impl LspClient {
    async fn semantic_locations(
        &self,
        request: &SemanticFactRequest,
    ) -> Result<Vec<FactLocation>, (FactUnavailable, Option<String>)> {
        let uri = crate::path_to_uri(&request.path)
            .map_err(|error| (FactUnavailable::Failure, Some(error.to_string())))?;
        let position = Position {
            line: request.line,
            character: request.character,
        };

        let failure = |error: LspError| match error {
            LspError::Timeout => (FactUnavailable::Timeout, None),
            other => (FactUnavailable::Failure, Some(other.to_string())),
        };

        match request.kind {
            SemanticFactKind::Definition => match self
                .goto_definition(&request.path, request.line, request.character)
                .await
            {
                Ok(Some(location)) => Ok(vec![location_to_fact(&location)]),
                Ok(None) => Ok(Vec::new()),
                Err(error) => Err(failure(error)),
            },
            SemanticFactKind::References => match self
                .references(&request.path, request.line, request.character)
                .await
            {
                Ok(locations) => Ok(locations.iter().map(location_to_fact).collect()),
                Err(error) => Err(failure(error)),
            },
            SemanticFactKind::TypeDefinition => match self
                .type_definition(&request.path, request.line, request.character)
                .await
            {
                Ok(locations) => Ok(locations.iter().map(location_to_fact).collect()),
                Err(error) => Err(failure(error)),
            },
            SemanticFactKind::Implementation => match self
                .goto_implementation(&request.path, request.line, request.character)
                .await
            {
                Ok(locations) => Ok(locations.iter().map(location_to_fact).collect()),
                Err(error) => Err(failure(error)),
            },
            SemanticFactKind::IncomingCalls => match self
                .incoming_calls(&request.path, request.line, request.character)
                .await
            {
                Ok(calls) => Ok(calls
                    .iter()
                    .map(|call| FactLocation {
                        file: uri_to_path(&call.from.uri),
                        start: FactPosition {
                            line: call.from.range.start.line,
                            character: call.from.range.start.character,
                        },
                        end: FactPosition {
                            line: call.from.range.end.line,
                            character: call.from.range.end.character,
                        },
                    })
                    .collect()),
                Err(error) => Err(failure(error)),
            },
            SemanticFactKind::OutgoingCalls => match self
                .outgoing_calls(&request.path, request.line, request.character)
                .await
            {
                Ok(calls) => Ok(calls
                    .iter()
                    .map(|call| FactLocation {
                        file: uri_to_path(&call.to.uri),
                        start: FactPosition {
                            line: call.to.range.start.line,
                            character: call.to.range.start.character,
                        },
                        end: FactPosition {
                            line: call.to.range.end.line,
                            character: call.to.range.end.character,
                        },
                    })
                    .collect()),
                Err(error) => Err(failure(error)),
            },
            SemanticFactKind::Supertypes => {
                let prepare = lsp_types::TypeHierarchyPrepareParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position,
                    },
                    work_done_progress_params: Default::default(),
                };
                let prepared = self
                    .request(
                        "textDocument/prepareTypeHierarchy",
                        serde_json::to_value(prepare)
                            .map_err(|error| (FactUnavailable::Failure, Some(error.to_string())))?,
                    )
                    .await
                    .map_err(failure)?;
                if prepared.is_null() {
                    return Ok(Vec::new());
                }
                let items: Vec<lsp_types::TypeHierarchyItem> = serde_json::from_value(prepared)
                    .map_err(|error| (FactUnavailable::Failure, Some(error.to_string())))?;
                let Some(item) = items.into_iter().next() else {
                    return Ok(Vec::new());
                };
                let params = lsp_types::TypeHierarchySupertypesParams {
                    item,
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                };
                let result = self
                    .request(
                        "typeHierarchy/supertypes",
                        serde_json::to_value(params)
                            .map_err(|error| (FactUnavailable::Failure, Some(error.to_string())))?,
                    )
                    .await
                    .map_err(failure)?;
                if result.is_null() {
                    return Ok(Vec::new());
                }
                let supertypes: Vec<lsp_types::TypeHierarchyItem> = serde_json::from_value(result)
                    .map_err(|error| (FactUnavailable::Failure, Some(error.to_string())))?;
                Ok(supertypes
                    .iter()
                    .map(|item| FactLocation {
                        file: uri_to_path(&item.uri),
                        start: FactPosition {
                            line: item.range.start.line,
                            character: item.range.start.character,
                        },
                        end: FactPosition {
                            line: item.range.end.line,
                            character: item.range.end.character,
                        },
                    })
                    .collect())
            }
        }
    }
}

impl LspClientRegistry {
    /// Answer a semantic fact request at one anchor. LSP is optional: a missing server,
    /// unsupported capability, timeout, or failure becomes an explicit `Unavailable`
    /// outcome instead of an error.
    pub async fn semantic_facts(&self, request: SemanticFactRequest) -> SemanticFactResponse {
        let path = canonicalize_or(&request.path);
        let Some(root) = self.root_for(&path).await else {
            return unavailable(
                FactUnavailable::NoServer,
                Some("no `lsp` configuration covers this file".to_string()),
            );
        };

        let Some(client) = self.client_for_file(&path).await else {
            return unavailable(
                FactUnavailable::Failure,
                Some("no LSP client is available (missing server, unsupported extension, or failed start)".to_string()),
            );
        };

        if !client
            .supports_capability(request.kind.capability_key())
            .await
        {
            return unavailable(
                FactUnavailable::Unsupported,
                Some(format!(
                    "server does not advertise {}",
                    request.kind.capability_key()
                )),
            );
        }

        let file_hash = match hash_file(&path) {
            Ok(hash) => hash,
            Err(error) => return unavailable(FactUnavailable::Failure, Some(error.to_string())),
        };
        let server_id = client.server_id().to_string();
        let key = FactCacheKey {
            server_id: server_id.clone(),
            path: path.clone(),
            line: request.line,
            character: request.character,
            kind: request.kind,
            file_hash,
        };

        if let Some(fact) = self.facts_cache.read().await.get(&key).cloned() {
            return SemanticFactResponse {
                outcome: SemanticFactOutcome::Exact(fact),
                cached: true,
            };
        }

        // Servers answer position queries against an open document, mirroring the tool
        // path. Failing to open is non-fatal; the query may still succeed.
        if let Ok(content) = std::fs::read_to_string(&path) {
            let language = crate::detect_language(&path);
            let _ = client.open_document(&path, &content, language).await;
        }

        let locations = match client.semantic_locations(&request).await {
            Ok(locations) => locations,
            Err((reason, detail)) => return unavailable(reason, detail),
        };
        let fact = SemanticFact {
            kind: request.kind,
            locations,
            confidence: FactConfidence::Exact,
            server_id,
            workspace_root: root,
            document_version: client.document_version(&path).await,
            file_hash,
        };

        // Do not cache an empty answer: a server may still be indexing when the
        // first anchor is asked. Empty results stay uncached so a later request can
        // resolve once the server is warm.
        if !fact.locations.is_empty() {
            let mut cache = self.facts_cache.write().await;
            // A changed file invalidates every cached anchor for that path.
            cache.retain(|existing, _| existing.path != path || existing.file_hash == file_hash);
            cache.insert(key, fact.clone());
        }

        SemanticFactResponse {
            outcome: SemanticFactOutcome::Exact(fact),
            cached: false,
        }
    }

    /// Clear cached facts for a file (or every file when `path` is None). Callers may
    /// use this after an edit; content-hash keying already invalidates stale entries.
    pub async fn invalidate_facts(&self, path: Option<&Path>) {
        let mut cache = self.facts_cache.write().await;
        match path {
            Some(path) => {
                let path = canonicalize_or(path);
                cache.retain(|key, _| key.path != path);
            }
            None => cache.clear(),
        }
    }
}

fn unavailable(reason: FactUnavailable, detail: Option<String>) -> SemanticFactResponse {
    SemanticFactResponse {
        outcome: SemanticFactOutcome::Unavailable { reason, detail },
        cached: false,
    }
}

/// Read a file's content hash. Exposed for consumers that want to align their own cache
/// invalidation with the fact boundary.
pub fn file_content_hash(path: &Path) -> std::io::Result<u64> {
    hash_file(path)
}

/// Best-effort bounded timeout used by the fact boundary. Kept here so consumers do not
/// need to know the client default.
pub const DEFAULT_FACT_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_keys_map_to_server_capabilities() {
        assert_eq!(
            SemanticFactKind::Definition.capability_key(),
            "definitionProvider"
        );
        assert_eq!(
            SemanticFactKind::Supertypes.capability_key(),
            "typeHierarchyProvider"
        );
        assert_eq!(
            SemanticFactKind::IncomingCalls.capability_key(),
            "callHierarchyProvider"
        );
    }

    #[tokio::test]
    async fn no_configured_workspace_is_explicitly_unavailable() {
        let registry = LspClientRegistry::new();
        let response = registry
            .semantic_facts(SemanticFactRequest::new(
                "/tmp/nowhere/main.rs",
                0,
                0,
                SemanticFactKind::Definition,
            ))
            .await;
        assert_eq!(
            response.outcome,
            SemanticFactOutcome::Unavailable {
                reason: FactUnavailable::NoServer,
                detail: Some("no `lsp` configuration covers this file".to_string()),
            }
        );
        assert!(!response.cached);
    }
}
