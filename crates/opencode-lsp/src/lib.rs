use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use lsp_types::{
    ClientCapabilities, Diagnostic, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, Range, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, VersionedTextDocumentIdentifier, WorkspaceFolder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, error};
use url::Url;

pub mod exact_facts;
pub mod facts;

pub use exact_facts::LspExactFactProvider;
pub use facts::{
    file_content_hash, FactConfidence, FactLocation, FactPosition, FactUnavailable, SemanticFact,
    SemanticFactKind, SemanticFactOutcome, SemanticFactRequest, SemanticFactResponse,
    DEFAULT_FACT_TIMEOUT,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyItem {
    pub name: String,
    pub kind: lsp_types::SymbolKind,
    #[serde(default)]
    pub tags: Option<Vec<lsp_types::SymbolTag>>,
    #[serde(default)]
    pub detail: Option<String>,
    pub uri: lsp_types::Uri,
    pub range: Range,
    pub selection_range: Range,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyPrepareParams {
    #[serde(flatten)]
    pub text_document_position_params: lsp_types::TextDocumentPositionParams,
    #[serde(flatten)]
    pub work_done_progress_params: lsp_types::WorkDoneProgressParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyIncomingCallsParams {
    pub item: CallHierarchyItem,
    #[serde(flatten)]
    pub work_done_progress_params: lsp_types::WorkDoneProgressParams,
    #[serde(flatten)]
    pub partial_result_params: lsp_types::PartialResultParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingCallsParams {
    pub item: CallHierarchyItem,
    #[serde(flatten)]
    pub work_done_progress_params: lsp_types::WorkDoneProgressParams,
    #[serde(flatten)]
    pub partial_result_params: lsp_types::PartialResultParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyIncomingCall {
    pub from: CallHierarchyItem,
    pub from_ranges: Vec<Range>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingCall {
    pub to: CallHierarchyItem,
    pub from_ranges: Vec<Range>,
}

fn path_to_uri(path: &Path) -> Result<lsp_types::Uri, LspError> {
    let url = Url::from_file_path(path)
        .map_err(|_| LspError::InitializeError("Invalid file path".to_string()))?;
    lsp_types::Uri::from_str(url.as_str())
        .map_err(|e| LspError::InitializeError(format!("Invalid URI: {}", e)))
}

fn uri_to_path(uri: &lsp_types::Uri) -> PathBuf {
    Url::parse(&uri.to_string())
        .ok()
        .and_then(|u| u.to_file_path().ok())
        .unwrap_or_default()
}

#[derive(Debug, Error)]
pub enum LspError {
    #[error("Failed to start LSP server: {0}")]
    ServerStartError(String),

    #[error("Failed to initialize LSP: {0}")]
    InitializeError(String),

    #[error("JSON-RPC error: {0}")]
    JsonRpcError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Server not initialized")]
    NotInitialized,

    #[error("Timeout waiting for response")]
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// One configured LSP server. `extensions` gates which files the server may handle;
/// an empty list means the server is considered for every extension. `disabled` keeps
/// the entry in config without starting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspServerConfig {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub extensions: Vec<String>,
    pub env: HashMap<String, String>,
    pub initialization_options: Option<Value>,
    pub disabled: bool,
}

impl LspServerConfig {
    /// True when this server should handle a file with the given extension.
    pub fn matches_extension(&self, extension: &str) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        let normalized = extension.trim_start_matches('.');
        self.extensions.iter().any(|value| {
            value
                .trim_start_matches('.')
                .eq_ignore_ascii_case(normalized)
        })
    }
}

/// Connection state reported by the `/lsp` status surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LspConnectionStatus {
    Connected,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspServerStatus {
    pub id: String,
    pub root: String,
    pub status: LspConnectionStatus,
}

pub struct LspClient {
    root: PathBuf,
    server_id: String,
    stdin: Arc<Mutex<ChildStdin>>,
    child: Arc<Mutex<Option<tokio::process::Child>>>,
    request_id: Arc<Mutex<u64>>,
    pending_responses:
        Arc<RwLock<HashMap<u64, tokio::sync::oneshot::Sender<Result<Value, LspError>>>>>,
    diagnostics: Arc<RwLock<HashMap<PathBuf, Vec<Diagnostic>>>>,
    file_versions: Arc<RwLock<HashMap<PathBuf, u32>>>,
    /// Raw `ServerCapabilities` from the initialize result. Kept as JSON so the fact
    /// boundary can negotiate methods (including ones the typed crate may not model).
    capabilities: Arc<RwLock<Option<Value>>>,
    request_timeout_ms: Arc<AtomicU64>,
    event_tx: broadcast::Sender<LspEvent>,
}

#[derive(Debug, Clone)]
pub enum LspEvent {
    Diagnostics { path: PathBuf, server_id: String },
}

impl LspClient {
    pub async fn start(config: LspServerConfig, root: PathBuf) -> Result<Self, LspError> {
        let mut child = Command::new(&config.command)
            .args(&config.args)
            .envs(&config.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| LspError::ServerStartError(e.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| LspError::ServerStartError("Failed to get stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LspError::ServerStartError("Failed to get stdout".to_string()))?;

        let (event_tx, _) = broadcast::channel(256);

        let client = Self {
            root,
            server_id: config.id.clone(),
            stdin: Arc::new(Mutex::new(stdin)),
            child: Arc::new(Mutex::new(Some(child))),
            request_id: Arc::new(Mutex::new(0)),
            pending_responses: Arc::new(RwLock::new(HashMap::new())),
            diagnostics: Arc::new(RwLock::new(HashMap::new())),
            file_versions: Arc::new(RwLock::new(HashMap::new())),
            capabilities: Arc::new(RwLock::new(None)),
            request_timeout_ms: Arc::new(AtomicU64::new(10_000)),
            event_tx,
        };

        let pending = client.pending_responses.clone();
        let diagnostics = client.diagnostics.clone();
        let server_id = config.id.clone();
        let event_tx_clone = client.event_tx.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);

            // Proper LSP framing: read `Content-Length` headers, then exactly that many
            // bytes. A server is not required to terminate the JSON body with a newline,
            // so line-splitting the stream deadlocks against valid servers.
            loop {
                let mut content_length = 0usize;
                loop {
                    let mut header = String::new();
                    match reader.read_line(&mut header).await {
                        Ok(0) => return,
                        Ok(_) => {}
                        Err(_) => return,
                    }
                    let header = header.trim_end_matches(['\r', '\n']);
                    if header.is_empty() {
                        break;
                    }
                    if let Some(value) = header.strip_prefix("Content-Length:") {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }

                if content_length == 0 {
                    continue;
                }

                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).await.is_err() {
                    return;
                }
                let line = String::from_utf8_lossy(&body).to_string();

                if let Ok(notification) = serde_json::from_str::<JsonRpcNotification>(&line) {
                    if notification.method == "textDocument/publishDiagnostics" {
                        if let Some(params) = notification.params {
                            if let Ok(diag_params) = serde_json::from_value::<
                                lsp_types::PublishDiagnosticsParams,
                            >(params)
                            {
                                let path = uri_to_path(&diag_params.uri);

                                diagnostics
                                    .write()
                                    .await
                                    .insert(path.clone(), diag_params.diagnostics);

                                let _ = event_tx_clone.send(LspEvent::Diagnostics {
                                    path,
                                    server_id: server_id.clone(),
                                });
                            }
                        }
                    }
                } else if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                    if let Some(sender) = pending.write().await.remove(&response.id) {
                        let result = if let Some(error) = response.error {
                            Err(LspError::JsonRpcError(error.message))
                        } else {
                            Ok(response.result.unwrap_or(Value::Null))
                        };
                        let _ = sender.send(result);
                    }
                }
            }
        });

        let mut client = client;
        client.initialize(config.initialization_options).await?;

        Ok(client)
    }

    async fn initialize(&mut self, initialization_options: Option<Value>) -> Result<(), LspError> {
        let workspace_uri = path_to_uri(&self.root)?;

        let params = InitializeParams {
            initialization_options,
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: workspace_uri,
                name: "workspace".to_string(),
            }]),
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };

        let result = self
            .request("initialize", serde_json::to_value(params)?)
            .await?;
        let capabilities = result.get("capabilities").cloned();
        *self.capabilities.write().await = capabilities;
        debug!(?result, "LSP initialized");

        self.notify("initialized", Value::Null).await?;

        Ok(())
    }

    async fn next_id(&self) -> u64 {
        let mut id = self.request_id.lock().await;
        *id += 1;
        *id
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value, LspError> {
        let id = self.next_id().await;
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.pending_responses.write().await.insert(id, tx);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let content = serde_json::to_string(&request)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(message.as_bytes()).await?;
        stdin.flush().await?;

        let timeout = Duration::from_millis(self.request_timeout_ms.load(Ordering::Relaxed));
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(LspError::Timeout),
            Err(_) => {
                self.pending_responses.write().await.remove(&id);
                Err(LspError::Timeout)
            }
        }
    }

    pub async fn notify(&self, method: &str, params: Value) -> Result<(), LspError> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: if params.is_null() { None } else { Some(params) },
        };

        let content = serde_json::to_string(&notification)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(message.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    pub async fn open_document(
        &self,
        path: &Path,
        content: &str,
        language_id: &str,
    ) -> Result<(), LspError> {
        let uri = path_to_uri(path)?;

        let version = {
            let mut versions = self.file_versions.write().await;
            let v = versions.entry(path.to_path_buf()).or_insert(0);
            *v
        };

        if version > 0 {
            let next_version = version + 1;
            self.file_versions
                .write()
                .await
                .insert(path.to_path_buf(), next_version);

            let params = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri,
                    version: next_version as i32,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: content.to_string(),
                }],
            };

            self.notify("textDocument/didChange", serde_json::to_value(params)?)
                .await?;
        } else {
            self.file_versions
                .write()
                .await
                .insert(path.to_path_buf(), 0);
            self.diagnostics.write().await.remove(path);

            let params = DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: language_id.to_string(),
                    version: 0,
                    text: content.to_string(),
                },
            };

            self.notify("textDocument/didOpen", serde_json::to_value(params)?)
                .await?;
        }

        Ok(())
    }

    pub async fn get_diagnostics(&self, path: &Path) -> Vec<Diagnostic> {
        self.diagnostics
            .read()
            .await
            .get(path)
            .cloned()
            .unwrap_or_default()
    }

    /// Returns all diagnostics from all files this LSP server has reported on.
    pub async fn get_all_diagnostics(&self) -> HashMap<PathBuf, Vec<Diagnostic>> {
        self.diagnostics.read().await.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LspEvent> {
        self.event_tx.subscribe()
    }

    pub async fn goto_definition(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<lsp_types::Location>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::GotoDefinitionParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("textDocument/definition", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(None);
        }

        let response: lsp_types::GotoDefinitionResponse = serde_json::from_value(result)?;
        match response {
            lsp_types::GotoDefinitionResponse::Scalar(loc) => Ok(Some(loc)),
            lsp_types::GotoDefinitionResponse::Array(locs) => Ok(locs.into_iter().next()),
            lsp_types::GotoDefinitionResponse::Link(links) => {
                Ok(links.into_iter().next().map(|l| lsp_types::Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                }))
            }
        }
    }

    pub async fn completion(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<Vec<lsp_types::CompletionItem>>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::CompletionParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };

        let result = self
            .request("textDocument/completion", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(None);
        }

        let response: lsp_types::CompletionResponse = serde_json::from_value(result)?;
        Ok(Some(match response {
            lsp_types::CompletionResponse::Array(items) => items,
            lsp_types::CompletionResponse::List(list) => list.items,
        }))
    }

    pub async fn references(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<lsp_types::Location>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::ReferenceParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let result = self
            .request("textDocument/references", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let locations: Vec<lsp_types::Location> = serde_json::from_value(result)?;
        Ok(locations)
    }

    pub async fn hover(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<lsp_types::Hover>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::HoverParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
        };

        let result = self
            .request("textDocument/hover", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(None);
        }

        let hover: lsp_types::Hover = serde_json::from_value(result)?;
        Ok(Some(hover))
    }

    pub async fn document_symbol(
        &self,
        path: &Path,
    ) -> Result<Vec<lsp_types::SymbolInformation>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("textDocument/documentSymbol", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let symbols: lsp_types::DocumentSymbolResponse = serde_json::from_value(result)?;
        Ok(match symbols {
            lsp_types::DocumentSymbolResponse::Flat(symbols) => symbols,
            lsp_types::DocumentSymbolResponse::Nested(nested) => nested
                .into_iter()
                .flat_map(|s| flatten_document_symbol(&s))
                .collect(),
        })
    }

    pub async fn workspace_symbol(
        &self,
        query: &str,
    ) -> Result<Vec<lsp_types::SymbolInformation>, LspError> {
        let params = lsp_types::WorkspaceSymbolParams {
            query: query.to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("workspace/symbol", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let symbols: Vec<lsp_types::SymbolInformation> = serde_json::from_value(result)?;
        Ok(symbols)
    }

    pub async fn goto_implementation(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<lsp_types::Location>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::request::GotoImplementationParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("textDocument/implementation", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let response: lsp_types::GotoDefinitionResponse = serde_json::from_value(result)?;
        Ok(match response {
            lsp_types::GotoDefinitionResponse::Scalar(loc) => vec![loc],
            lsp_types::GotoDefinitionResponse::Array(locs) => locs,
            lsp_types::GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| lsp_types::Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
        })
    }

    pub async fn type_definition(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<lsp_types::Location>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: lsp_types::Position { line, character },
        };

        let result = self
            .request("textDocument/typeDefinition", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let response: lsp_types::GotoDefinitionResponse = serde_json::from_value(result)?;
        Ok(match response {
            lsp_types::GotoDefinitionResponse::Scalar(loc) => vec![loc],
            lsp_types::GotoDefinitionResponse::Array(locs) => locs,
            lsp_types::GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| lsp_types::Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
        })
    }

    pub async fn rename(
        &self,
        path: &Path,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<Option<lsp_types::WorkspaceEdit>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::RenameParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        };

        let result = self
            .request("textDocument/rename", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(None);
        }

        let edit: lsp_types::WorkspaceEdit = serde_json::from_value(result)?;
        Ok(Some(edit))
    }

    pub async fn prepare_call_hierarchy(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<CallHierarchyItem>, LspError> {
        let uri = path_to_uri(path)?;

        let params = CallHierarchyPrepareParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
        };

        let result = self
            .request(
                "textDocument/prepareCallHierarchy",
                serde_json::to_value(params)?,
            )
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let items: Vec<CallHierarchyItem> = serde_json::from_value(result)?;
        Ok(items)
    }

    pub async fn incoming_calls(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<CallHierarchyIncomingCall>, LspError> {
        let items = self.prepare_call_hierarchy(path, line, character).await?;

        if items.is_empty() {
            return Ok(vec![]);
        }

        let item = &items[0];
        let params = CallHierarchyIncomingCallsParams {
            item: item.clone(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("callHierarchy/incomingCalls", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let calls: Vec<CallHierarchyIncomingCall> = serde_json::from_value(result)?;
        Ok(calls)
    }

    pub async fn outgoing_calls(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<CallHierarchyOutgoingCall>, LspError> {
        let items = self.prepare_call_hierarchy(path, line, character).await?;

        if items.is_empty() {
            return Ok(vec![]);
        }

        let item = &items[0];
        let params = CallHierarchyOutgoingCallsParams {
            item: item.clone(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("callHierarchy/outgoingCalls", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let calls: Vec<CallHierarchyOutgoingCall> = serde_json::from_value(result)?;
        Ok(calls)
    }

    /// Workspace root this client was started for.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Configured server id this client was started for.
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// Timeout applied to every JSON-RPC request.
    pub fn set_request_timeout(&self, timeout: Duration) {
        self.request_timeout_ms
            .store(timeout.as_millis().max(1) as u64, Ordering::Relaxed);
    }

    /// Current tracked document version for a file, if it was opened.
    pub async fn document_version(&self, path: &Path) -> Option<i32> {
        self.file_versions
            .read()
            .await
            .get(path)
            .map(|version| *version as i32)
    }

    /// True when the server advertised a truthy capability key.
    pub async fn supports_capability(&self, key: &str) -> bool {
        self.capabilities
            .read()
            .await
            .as_ref()
            .and_then(|capabilities| capabilities.get(key))
            .map(|value| !value.is_null() && value != &Value::Bool(false))
            .unwrap_or(false)
    }

    /// Best-effort graceful shutdown, then kill. Safe to call more than once.
    pub async fn shutdown(&self) {
        let _ = self.notify("shutdown", Value::Null).await;
        let _ = self.notify("exit", Value::Null).await;
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }
}

#[allow(deprecated)]
fn flatten_document_symbol(
    symbol: &lsp_types::DocumentSymbol,
) -> Vec<lsp_types::SymbolInformation> {
    let mut result = vec![];

    result.push(lsp_types::SymbolInformation {
        name: symbol.name.clone(),
        kind: symbol.kind,
        tags: symbol.tags.clone(),
        deprecated: symbol.deprecated,
        location: lsp_types::Location {
            uri: path_to_uri(&std::path::PathBuf::new())
                .unwrap_or_else(|_| lsp_types::Uri::from_str("file:///").unwrap()),
            range: symbol.selection_range,
        },
        container_name: symbol.detail.clone(),
    });

    if let Some(children) = &symbol.children {
        for child in children {
            result.extend(flatten_document_symbol(child));
        }
    }

    result
}

pub struct LspClientRegistry {
    clients: RwLock<HashMap<String, Arc<LspClient>>>,
    configs: RwLock<HashMap<PathBuf, Vec<LspServerConfig>>>,
    broken: RwLock<HashSet<String>>,
    request_timeout: RwLock<Duration>,
    pub(crate) facts_cache: RwLock<HashMap<facts::FactCacheKey, facts::SemanticFact>>,
}

fn canonicalize_or(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn client_key(root: &Path, server_id: &str) -> String {
    format!("{}\u{0}{}", root.display(), server_id)
}

impl LspClientRegistry {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            configs: RwLock::new(HashMap::new()),
            broken: RwLock::new(HashSet::new()),
            request_timeout: RwLock::new(Duration::from_secs(10)),
            facts_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Set the request timeout applied to all clients, including already-running ones.
    pub async fn set_request_timeout(&self, timeout: Duration) {
        *self.request_timeout.write().await = timeout;
        for client in self.clients.read().await.values() {
            client.set_request_timeout(timeout);
        }
    }

    /// Register a client directly. Retained for tests and explicit wiring; the runtime
    /// uses `configure` + `client_for_file` for lazy `(root, server)` spawning.
    pub async fn register(&self, id: String, client: Arc<LspClient>) {
        self.clients.write().await.insert(id, client);
    }

    pub async fn get(&self, id: &str) -> Option<Arc<LspClient>> {
        self.clients.read().await.get(id).cloned()
    }

    pub async fn list(&self) -> Vec<(String, Arc<LspClient>)> {
        self.clients
            .read()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Install the server definitions for a workspace root. Idempotent: configuring the
    /// same root with the same definitions keeps live clients. A changed definition set
    /// shuts down that root's clients and drops its broken-server marks so the next
    /// request retries lazily.
    pub async fn configure(&self, root: PathBuf, servers: Vec<LspServerConfig>) {
        let root = canonicalize_or(&root);
        {
            let configs = self.configs.read().await;
            if configs.get(&root) == Some(&servers) {
                return;
            }
        }
        self.shutdown_root(&root).await;
        self.configs.write().await.insert(root, servers);
    }

    async fn shutdown_root(&self, root: &Path) {
        let prefix = format!("{}\u{0}", root.display());
        let removed: Vec<Arc<LspClient>> = {
            let mut clients = self.clients.write().await;
            let keys: Vec<String> = clients
                .keys()
                .filter(|key| key.starts_with(&prefix))
                .cloned()
                .collect();
            keys.into_iter()
                .filter_map(|key| clients.remove(&key))
                .collect()
        };
        for client in removed {
            client.shutdown().await;
        }
        let mut broken = self.broken.write().await;
        broken.retain(|key| !key.starts_with(&prefix));
    }

    /// Shut down every client. Used on server shutdown and by tests.
    pub async fn shutdown_all(&self) {
        let removed: Vec<Arc<LspClient>> = {
            let mut clients = self.clients.write().await;
            clients.drain().map(|(_, client)| client).collect()
        };
        for client in removed {
            client.shutdown().await;
        }
        self.broken.write().await.clear();
    }

    /// Longest configured root that contains `path`, if any. LSP is opt-in: a file
    /// outside every configured workspace spawns nothing.
    pub async fn root_for(&self, path: &Path) -> Option<PathBuf> {
        let path = canonicalize_or(path);
        self.configs
            .read()
            .await
            .keys()
            .filter(|root| path.starts_with(root))
            .max_by_key(|root| root.components().count())
            .cloned()
    }

    fn matching_servers<'a>(
        servers: &'a [LspServerConfig],
        path: &Path,
    ) -> Vec<&'a LspServerConfig> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        servers
            .iter()
            .filter(|server| !server.disabled && !server.command.is_empty())
            .filter(|server| server.matches_extension(extension))
            .collect()
    }

    /// Lazily spawn (or reuse) every configured server that can handle `path`. A server
    /// that fails to start is suppressed for this registry lifetime; the caller simply
    /// gets no client and the task proceeds without LSP.
    pub async fn ensure_for_file(&self, path: &Path) -> Result<Vec<Arc<LspClient>>, LspError> {
        let Some(root) = self.root_for(path).await else {
            return Ok(Vec::new());
        };
        let servers = {
            let configs = self.configs.read().await;
            Self::matching_servers(configs.get(&root).map(Vec::as_slice).unwrap_or(&[]), path)
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        };

        let mut clients = Vec::new();
        for server in servers {
            let key = client_key(&root, &server.id);
            if let Some(client) = self.clients.read().await.get(&key).cloned() {
                clients.push(client);
                continue;
            }
            if self.broken.read().await.contains(&key) {
                continue;
            }
            match LspClient::start(server.clone(), root.clone()).await {
                Ok(client) => {
                    client.set_request_timeout(*self.request_timeout.read().await);
                    let client = Arc::new(client);
                    self.clients.write().await.insert(key, client.clone());
                    clients.push(client);
                }
                Err(error) => {
                    self.broken.write().await.insert(key);
                    error!(
                        server = %server.id,
                        root = %root.display(),
                        %error,
                        "LSP server failed to start; suppressing for this registry"
                    );
                }
            }
        }
        Ok(clients)
    }

    /// First live client for `path`, spawning matching configured servers if needed.
    pub async fn client_for_file(&self, path: &Path) -> Option<Arc<LspClient>> {
        self.ensure_for_file(path)
            .await
            .unwrap_or_default()
            .into_iter()
            .next()
    }

    /// True when at least one configured server can handle `path`. Spawns lazily.
    pub async fn has_clients(&self, path: &Path) -> bool {
        !self
            .ensure_for_file(path)
            .await
            .unwrap_or_default()
            .is_empty()
    }

    /// Opens or refreshes a file in all matching LSP clients.
    /// Mirrors the TS `LSP.touchFile(input, waitForDiagnostics)`.
    pub async fn touch_file(
        &self,
        path: &Path,
        wait_for_diagnostics: bool,
    ) -> Result<(), LspError> {
        let language = detect_language(path);
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(LspError::IoError)?;

        let matching = self.ensure_for_file(path).await?;

        for client in &matching {
            if let Err(e) = client.open_document(path, &content, language).await {
                error!("Failed to touch file {:?} in LSP: {}", path, e);
            }
        }

        if wait_for_diagnostics && !matching.is_empty() {
            // Give LSP servers a moment to produce diagnostics.
            // The TS version uses client.waitForDiagnostics which listens
            // for a publishDiagnostics notification. We approximate this
            // with a short sleep, since the broadcast-based event system
            // already stores diagnostics as they arrive.
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        Ok(())
    }

    /// Actual connected servers and roots, plus servers that failed to start. Configured
    /// but not-yet-started servers are not "connected" and are omitted.
    pub async fn status(&self) -> Vec<LspServerStatus> {
        let configs = self.configs.read().await.clone();
        let clients = self.clients.read().await;
        let broken = self.broken.read().await;
        let mut statuses = Vec::new();
        for (root, servers) in &configs {
            for server in servers {
                let key = client_key(root, &server.id);
                let status = if clients.contains_key(&key) {
                    Some(LspConnectionStatus::Connected)
                } else if broken.contains(&key) {
                    Some(LspConnectionStatus::Error)
                } else {
                    None
                };
                if let Some(status) = status {
                    statuses.push(LspServerStatus {
                        id: server.id.clone(),
                        root: root.display().to_string(),
                        status,
                    });
                }
            }
        }
        statuses.sort_by(|a, b| a.root.cmp(&b.root).then(a.id.cmp(&b.id)));
        statuses
    }
}

/// Find a plausible workspace root by walking up from `path` looking for a project
/// marker. Falls back to `path`'s nearest existing ancestor (or `path` itself).
pub fn detect_root(path: &Path) -> PathBuf {
    const MARKERS: &[&str] = &[
        ".git",
        ".hg",
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        "setup.py",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
    ];
    let start = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    let mut current = Some(start);
    while let Some(dir) = current {
        if MARKERS.iter().any(|marker| dir.join(marker).exists()) {
            return canonicalize_or(dir);
        }
        current = dir.parent();
    }
    canonicalize_or(start)
}

impl Default for LspClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn detect_language(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("ts") => "typescript",
        Some("tsx") => "typescriptreact",
        Some("js") => "javascript",
        Some("jsx") => "javascriptreact",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("c") => "c",
        Some("cpp") | Some("cc") | Some("cxx") => "cpp",
        Some("h") | Some("hpp") => "cpp",
        Some("rb") => "ruby",
        Some("php") => "php",
        Some("swift") => "swift",
        Some("kt") => "kotlin",
        Some("scala") => "scala",
        Some("lua") => "lua",
        Some("json") => "json",
        Some("yaml") | Some("yml") => "yaml",
        Some("toml") => "toml",
        Some("md") => "markdown",
        Some("html") => "html",
        Some("css") => "css",
        Some("scss") => "scss",
        _ => "plaintext",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("main.rs")), "rust");
        assert_eq!(detect_language(Path::new("index.ts")), "typescript");
        assert_eq!(detect_language(Path::new("app.tsx")), "typescriptreact");
        assert_eq!(detect_language(Path::new("main.py")), "python");
        assert_eq!(detect_language(Path::new("main.go")), "go");
        assert_eq!(detect_language(Path::new("unknown.xyz")), "plaintext");
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "initialize".to_string(),
            params: Value::Null,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn test_json_rpc_response_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_has_clients_empty_registry() {
        let registry = LspClientRegistry::new();
        assert!(!registry.has_clients(Path::new("main.rs")).await);
        assert!(!registry.has_clients(Path::new("index.ts")).await);
    }

    #[tokio::test]
    async fn test_registry_default() {
        let registry = LspClientRegistry::default();
        let clients = registry.list().await;
        assert!(clients.is_empty());
    }

    fn server(id: &str, extensions: &[&str], command: &str) -> LspServerConfig {
        LspServerConfig {
            id: id.to_string(),
            command: command.to_string(),
            args: Vec::new(),
            extensions: extensions.iter().map(|ext| ext.to_string()).collect(),
            env: HashMap::new(),
            initialization_options: None,
            disabled: false,
        }
    }

    #[test]
    fn extension_gating_accepts_dot_or_bare_and_empty_matches_all() {
        let rust = server("rust", &[".rs"], "rust-analyzer");
        assert!(rust.matches_extension("rs"));
        assert!(rust.matches_extension(".rs"));
        assert!(!rust.matches_extension("py"));

        let any = server("any", &[], "server");
        assert!(any.matches_extension("anything"));
    }

    #[tokio::test]
    async fn unconfigured_file_spawns_nothing() {
        let registry = LspClientRegistry::new();
        assert!(registry
            .ensure_for_file(Path::new("/tmp/nowhere/main.rs"))
            .await
            .unwrap()
            .is_empty());
        assert!(
            !registry
                .has_clients(Path::new("/tmp/nowhere/main.rs"))
                .await
        );
        assert!(registry.status().await.is_empty());
    }

    #[tokio::test]
    async fn broken_server_is_suppressed_and_reported_as_error() {
        let root = std::env::temp_dir().join("opencode-lsp-broken-server-test");
        let _ = std::fs::create_dir_all(&root);
        let registry = LspClientRegistry::new();
        registry
            .configure(
                root.clone(),
                vec![server("broken", &[".rs"], "/nonexistent/lsp-server-xyz")],
            )
            .await;

        let file = root.join("main.rs");
        assert!(registry.ensure_for_file(&file).await.unwrap().is_empty());
        assert!(!registry.has_clients(&file).await);

        let status = registry.status().await;
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].id, "broken");
        assert_eq!(status[0].status, LspConnectionStatus::Error);

        // A second request does not retry the broken server.
        assert!(registry.ensure_for_file(&file).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn disabled_server_is_never_started() {
        let root = std::env::temp_dir().join("opencode-lsp-disabled-server-test");
        let _ = std::fs::create_dir_all(&root);
        let registry = LspClientRegistry::new();
        let mut disabled = server("disabled", &[".rs"], "/nonexistent/lsp-server-xyz");
        disabled.disabled = true;
        registry.configure(root.clone(), vec![disabled]).await;

        assert!(registry
            .ensure_for_file(&root.join("main.rs"))
            .await
            .unwrap()
            .is_empty());
        assert!(registry.status().await.is_empty());
    }

    #[tokio::test]
    async fn root_for_picks_the_longest_containing_root() {
        let parent = std::env::temp_dir().join("opencode-lsp-root-parent");
        let child = parent.join("nested");
        let _ = std::fs::create_dir_all(&child);
        let registry = LspClientRegistry::new();
        registry.configure(parent.clone(), Vec::new()).await;
        registry.configure(child.clone(), Vec::new()).await;

        let file = child.join("main.rs");
        let root = registry.root_for(&file).await.unwrap();
        assert_eq!(
            root,
            child.canonicalize().unwrap_or(child),
            "the nested configured root should win"
        );
    }

    #[tokio::test]
    async fn shutdown_all_clears_clients() {
        let registry = LspClientRegistry::new();
        registry.shutdown_all().await;
        assert!(registry.list().await.is_empty());
    }

    fn python3_available() -> bool {
        std::process::Command::new("python3")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    const FAKE_LSP_SERVER: &str = r#"#!/usr/bin/env python3
import json
import sys

def read_message():
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line in (b"\r\n", b"\n"):
            break
        key, value = line.decode("utf-8").split(":", 1)
        headers[key.strip().lower()] = value.strip()
    length = int(headers.get("content-length", "0"))
    return json.loads(sys.stdin.buffer.read(length))

def send(payload):
    data = json.dumps(payload).encode("utf-8")
    sys.stdout.buffer.write(f"Content-Length: {len(data)}\r\n\r\n".encode("utf-8"))
    sys.stdout.buffer.write(data)
    sys.stdout.buffer.flush()

while True:
    message = read_message()
    if message is None:
        break
    if "id" not in message:
        continue
    if message.get("method") == "initialize":
        send({"jsonrpc": "2.0", "id": message["id"], "result": {"capabilities": {"definitionProvider": True}}})
    else:
        send({"jsonrpc": "2.0", "id": message["id"], "result": None})
"#;

    const FAKE_FACT_SERVER: &str = r#"#!/usr/bin/env python3
import json
import pathlib
import sys
import time

TARGET = sys.argv[1] if len(sys.argv) > 1 else ""
MODE = sys.argv[2] if len(sys.argv) > 2 else "answer"

def read_message():
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line in (b"\r\n", b"\n"):
            break
        key, value = line.decode("utf-8").split(":", 1)
        headers[key.strip().lower()] = value.strip()
    length = int(headers.get("content-length", "0"))
    return json.loads(sys.stdin.buffer.read(length))

def send(payload):
    data = json.dumps(payload).encode("utf-8")
    sys.stdout.buffer.write(f"Content-Length: {len(data)}\r\n\r\n".encode("utf-8"))
    sys.stdout.buffer.write(data)
    sys.stdout.buffer.flush()

while True:
    message = read_message()
    if message is None:
        break
    if "id" not in message:
        continue
    method = message.get("method")
    if method == "initialize":
        send({"jsonrpc": "2.0", "id": message["id"], "result": {"capabilities": {
            "definitionProvider": True,
            "referencesProvider": True,
            "implementationProvider": True,
            "callHierarchyProvider": True,
        }}})
    elif method == "textDocument/definition":
        if MODE == "hang":
            time.sleep(3600)
        if MODE == "none":
            send({"jsonrpc": "2.0", "id": message["id"], "result": None})
        else:
            send({"jsonrpc": "2.0", "id": message["id"], "result": [{
                "uri": pathlib.Path(TARGET).as_uri(),
                "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}},
            }]})
    else:
        send({"jsonrpc": "2.0", "id": message["id"], "result": None})
"#;

    async fn configure_fact_server(
        registry: &LspClientRegistry,
        root: &std::path::Path,
        target: &std::path::Path,
        mode: &str,
    ) {
        let script = root.join("fake_fact_server.py");
        std::fs::write(&script, FAKE_FACT_SERVER).unwrap();
        registry
            .configure(
                root.to_path_buf(),
                vec![LspServerConfig {
                    id: "fake".to_string(),
                    command: "python3".to_string(),
                    args: vec![
                        script.display().to_string(),
                        target.display().to_string(),
                        mode.to_string(),
                    ],
                    extensions: vec![".rs".to_string()],
                    env: HashMap::new(),
                    initialization_options: None,
                    disabled: false,
                }],
            )
            .await;
    }

    fn fact_test_root(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("opencode-lsp-fact-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[tokio::test]
    async fn semantic_facts_are_exact_cached_and_invalidated_by_content() {
        if !python3_available() {
            return;
        }
        let root = fact_test_root("cache");
        let main = root.join("main.rs");
        let target = root.join("def.rs");
        std::fs::write(&main, "fn main() {}\n").unwrap();
        std::fs::write(&target, "struct Def;\n").unwrap();

        let registry = LspClientRegistry::new();
        configure_fact_server(&registry, &root, &target, "answer").await;

        let request = SemanticFactRequest::new(&main, 0, 0, SemanticFactKind::Definition);
        let first = registry.semantic_facts(request.clone()).await;
        assert!(!first.cached);
        let fact = match first.outcome {
            SemanticFactOutcome::Exact(fact) => fact,
            other => panic!("expected exact fact, got {other:?}"),
        };
        assert_eq!(fact.kind, SemanticFactKind::Definition);
        assert_eq!(fact.confidence, FactConfidence::Exact);
        assert_eq!(fact.server_id, "fake");
        assert_eq!(fact.locations.len(), 1);
        assert!(fact.locations[0].file.ends_with("def.rs"));
        assert_ne!(fact.file_hash, 0);

        let second = registry.semantic_facts(request.clone()).await;
        assert!(second.cached, "identical content should hit the cache");

        std::fs::write(&main, "fn main() { let _ = 1; }\n").unwrap();
        let third = registry.semantic_facts(request).await;
        assert!(!third.cached, "changed content must miss the cache");

        registry.shutdown_all().await;
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn unsupported_capability_is_explicit_and_does_not_fail() {
        if !python3_available() {
            return;
        }
        let root = fact_test_root("unsupported");
        let main = root.join("main.rs");
        let target = root.join("def.rs");
        std::fs::write(&main, "fn main() {}\n").unwrap();
        std::fs::write(&target, "struct Def;\n").unwrap();

        let registry = LspClientRegistry::new();
        configure_fact_server(&registry, &root, &target, "answer").await;

        // The fake server does not advertise typeDefinitionProvider.
        let response = registry
            .semantic_facts(SemanticFactRequest::new(
                &main,
                0,
                0,
                SemanticFactKind::TypeDefinition,
            ))
            .await;
        assert!(matches!(
            response.outcome,
            SemanticFactOutcome::Unavailable {
                reason: FactUnavailable::Unsupported,
                ..
            }
        ));

        registry.shutdown_all().await;
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn hung_server_times_out_without_failing_the_caller() {
        if !python3_available() {
            return;
        }
        let root = fact_test_root("timeout");
        let main = root.join("main.rs");
        let target = root.join("def.rs");
        std::fs::write(&main, "fn main() {}\n").unwrap();
        std::fs::write(&target, "struct Def;\n").unwrap();

        let registry = LspClientRegistry::new();
        registry
            .set_request_timeout(Duration::from_millis(250))
            .await;
        configure_fact_server(&registry, &root, &target, "hang").await;

        let response = registry
            .semantic_facts(SemanticFactRequest::new(
                &main,
                0,
                0,
                SemanticFactKind::Definition,
            ))
            .await;
        assert!(matches!(
            response.outcome,
            SemanticFactOutcome::Unavailable {
                reason: FactUnavailable::Timeout,
                ..
            }
        ));

        registry.shutdown_all().await;
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn configured_fake_server_starts_lazily_and_answers_requests() {
        if !python3_available() {
            return;
        }
        let root = std::env::temp_dir().join(format!(
            "opencode-lsp-fake-server-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let script = root.join("fake_lsp.py");
        std::fs::write(&script, FAKE_LSP_SERVER).unwrap();
        let file = root.join("main.rs");
        std::fs::write(&file, "fn main() {}\n").unwrap();

        let registry = LspClientRegistry::new();
        registry
            .configure(
                root.clone(),
                vec![LspServerConfig {
                    id: "fake".to_string(),
                    command: "python3".to_string(),
                    args: vec![script.display().to_string()],
                    extensions: vec![".rs".to_string()],
                    env: HashMap::new(),
                    initialization_options: None,
                    disabled: false,
                }],
            )
            .await;

        let client = registry
            .client_for_file(&file)
            .await
            .expect("configured fake server should start for a matching file");
        let definition = client.goto_definition(&file, 0, 0).await.unwrap();
        assert!(definition.is_none(), "fake server returns no definition");

        let status = registry.status().await;
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].status, LspConnectionStatus::Connected);

        registry.shutdown_all().await;
        assert!(registry.list().await.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
    #[tokio::test]
    #[ignore = "requires rust-analyzer; slow real-server smoke test"]
    async fn real_rust_analyzer_definition_smoke() {
        let available = std::process::Command::new("rust-analyzer")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if !available {
            return;
        }
        let root = fact_test_root("rust-analyzer");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"lsp-smoke\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("src/main.rs"),
            "mod def;\nfn main() { def::helper(); }\n",
        )
        .unwrap();
        std::fs::write(root.join("src/def.rs"), "pub fn helper() {}\n").unwrap();

        let registry = LspClientRegistry::new();
        registry.set_request_timeout(Duration::from_secs(60)).await;
        registry
            .configure(
                root.clone(),
                vec![LspServerConfig {
                    id: "rust-analyzer".to_string(),
                    command: "rust-analyzer".to_string(),
                    args: Vec::new(),
                    extensions: vec![".rs".to_string()],
                    env: HashMap::new(),
                    initialization_options: None,
                    disabled: false,
                }],
            )
            .await;

        let main = root.join("src/main.rs");
        let response = registry
            .semantic_facts(SemanticFactRequest::new(
                &main,
                1,
                13,
                SemanticFactKind::Definition,
            ))
            .await;
        match response.outcome {
            SemanticFactOutcome::Exact(fact) => {
                assert_eq!(fact.confidence, FactConfidence::Exact);
                assert_eq!(fact.server_id, "rust-analyzer");
                assert!(
                    fact.locations
                        .iter()
                        .any(|loc| loc.file.ends_with("def.rs")),
                    "expected definition in def.rs, got {:?}",
                    fact.locations
                );
            }
            other => panic!("expected exact definition from rust-analyzer, got {other:?}"),
        }

        registry.shutdown_all().await;
        let _ = std::fs::remove_dir_all(&root);
    }
}
