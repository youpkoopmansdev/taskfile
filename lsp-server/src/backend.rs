use std::collections::HashMap;
use std::sync::RwLock;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::parser;
use crate::parser::ast::Ast;

pub struct Backend {
    client: Client,
    documents: RwLock<HashMap<Url, (String, Ast)>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: RwLock::new(HashMap::new()),
        }
    }

    fn reparse_and_publish(&self, uri: Url, text: String) {
        let ast = parser::parse(&text);
        let diagnostics = self.convert_diagnostics(&ast);
        let docs = self.documents.read().unwrap();
        let version = None; // We don't track versions
        drop(docs);

        {
            let mut docs = self.documents.write().unwrap();
            docs.insert(uri.clone(), (text, ast));
        }

        let client = self.client.clone();
        tokio::spawn(async move {
            client.publish_diagnostics(uri, diagnostics, version).await;
        });
    }

    fn convert_diagnostics(&self, ast: &Ast) -> Vec<tower_lsp::lsp_types::Diagnostic> {
        ast.diagnostics
            .iter()
            .map(|d| {
                let severity = match d.severity {
                    parser::ast::DiagnosticSeverity::Error => {
                        Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR)
                    }
                    parser::ast::DiagnosticSeverity::Warning => {
                        Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING)
                    }
                };
                tower_lsp::lsp_types::Diagnostic {
                    range: span_to_range(&d.span),
                    severity,
                    message: d.message.clone(),
                    source: Some("taskfile".to_string()),
                    ..Default::default()
                }
            })
            .collect()
    }

    fn get_ast(&self, uri: &Url) -> Option<Ast> {
        let docs = self.documents.read().unwrap();
        docs.get(uri).map(|(_, ast)| ast.clone())
    }

    fn get_text(&self, uri: &Url) -> Option<String> {
        let docs = self.documents.read().unwrap();
        docs.get(uri).map(|(text, _)| text.clone())
    }

    fn find_word_at_position(&self, uri: &Url, pos: Position) -> Option<String> {
        let text = self.get_text(uri)?;
        let lines: Vec<&str> = text.lines().collect();
        let line = lines.get(pos.line as usize)?;
        let col = pos.character as usize;

        if col > line.len() {
            return None;
        }

        // Find word boundaries around the cursor
        let start = line[..col]
            .rfind(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .map(|p| p + 1)
            .unwrap_or(0);
        let end = line[col..]
            .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .map(|p| p + col)
            .unwrap_or(line.len());

        if start >= end {
            return None;
        }

        Some(line[start..end].to_string())
    }

    fn is_in_depends_context(&self, uri: &Url, pos: Position) -> bool {
        let text = match self.get_text(uri) {
            Some(t) => t,
            None => return false,
        };
        let lines: Vec<&str> = text.lines().collect();
        let line = match lines.get(pos.line as usize) {
            Some(l) => *l,
            None => return false,
        };
        let col = pos.character as usize;
        let before_cursor = &line[..col.min(line.len())];
        // Check if we're inside depends=[...] or depends_parallel=[...]
        let has_depends_open =
            before_cursor.contains("depends=[") || before_cursor.contains("depends_parallel=[");
        if !has_depends_open {
            return false;
        }
        // Make sure we haven't closed the bracket yet
        let last_open = before_cursor
            .rfind("depends=[")
            .or_else(|| before_cursor.rfind("depends_parallel=["));
        if let Some(open_pos) = last_open {
            let after_open = &before_cursor[open_pos..];
            // Count brackets
            let opens = after_open.chars().filter(|c| *c == '[').count();
            let closes = after_open.chars().filter(|c| *c == ']').count();
            return opens > closes;
        }
        false
    }

    fn find_include_path_at_position(&self, uri: &Url, pos: Position) -> Option<String> {
        let ast = self.get_ast(uri)?;
        for inc in &ast.includes {
            if pos.line == inc.span.start_line && pos.line == inc.span.end_line {
                return Some(inc.path.clone());
            }
        }
        None
    }

    fn build_hover_for_task(&self, task: &crate::parser::ast::Task) -> String {
        let mut parts = Vec::new();

        parts.push(format!("**task** `{}`", task.name));

        if let Some(ref desc) = task.description {
            parts.push(desc.clone());
        }

        if !task.params.is_empty() {
            let param_strs: Vec<String> = task
                .params
                .iter()
                .map(|p| match &p.default {
                    Some(d) => format!("{}=\"{}\"", p.name, d),
                    None => p.name.clone(),
                })
                .collect();
            parts.push(format!("**Parameters:** {}", param_strs.join(", ")));
        }

        if !task.dependencies.is_empty() {
            let dep_names: Vec<&str> = task.dependencies.iter().map(|d| d.name.as_str()).collect();
            parts.push(format!("**Depends:** {}", dep_names.join(", ")));
        }

        if !task.parallel_dependencies.is_empty() {
            let dep_names: Vec<&str> = task
                .parallel_dependencies
                .iter()
                .map(|d| d.name.as_str())
                .collect();
            parts.push(format!("**Parallel depends:** {}", dep_names.join(", ")));
        }

        parts.join("\n\n")
    }
}

fn span_to_range(span: &crate::parser::ast::Span) -> Range {
    Range {
        start: Position {
            line: span.start_line,
            character: span.start_col,
        },
        end: Position {
            line: span.end_line,
            character: span.end_col,
        },
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "[".to_string(),
                        ",".to_string(),
                        " ".to_string(),
                        "@".to_string(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Taskfile LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.reparse_and_publish(uri, text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.reparse_and_publish(uri, change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut docs = self.documents.write().unwrap();
        docs.remove(&uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let mut items = Vec::new();

        // Check if we're in a depends context — offer task names
        if self.is_in_depends_context(uri, pos) {
            if let Some(ast) = self.get_ast(uri) {
                for task in &ast.tasks {
                    items.push(CompletionItem {
                        label: task.name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: task.description.clone(),
                        ..Default::default()
                    });
                }
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        // Check the current line content to decide what to suggest
        let text = self.get_text(uri);
        let line_text = text
            .as_ref()
            .and_then(|t| t.lines().nth(pos.line as usize).map(|s| s.to_string()));
        let before_cursor = line_text
            .as_ref()
            .map(|l| &l[..((pos.character as usize).min(l.len()))])
            .unwrap_or("");
        let trimmed = before_cursor.trim_start();

        // If the line is empty or just starting, offer top-level keywords
        let is_toplevel = trimmed.is_empty()
            || trimmed.starts_with("ta")
            || trimmed.starts_with("ex")
            || trimmed.starts_with("al")
            || trimmed.starts_with("in")
            || trimmed.starts_with("do")
            || trimmed.starts_with('@');

        if is_toplevel {
            let keywords = [
                ("task", "Define a new task"),
                ("export", "Export an environment variable"),
                ("alias", "Define a shell alias"),
                ("include", "Include another Taskfile"),
                ("dotenv", "Load a .env file"),
                ("@description", "Set task description"),
                ("@confirm", "Require confirmation before running"),
            ];
            for (kw, detail) in keywords {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(detail.to_string()),
                    ..Default::default()
                });
            }
        }

        // If on a task header line, offer depends keywords
        if trimmed.starts_with("task ") {
            for kw in ["depends", "depends_parallel"] {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    insert_text: Some(format!("{}=[$0]", kw)),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                });
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let word = match self.find_word_at_position(uri, pos) {
            Some(w) => w,
            None => return Ok(None),
        };

        let ast = match self.get_ast(uri) {
            Some(a) => a,
            None => return Ok(None),
        };

        // Find a matching task
        for task in &ast.tasks {
            if task.name == word {
                let content = self.build_hover_for_task(task);
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: content,
                    }),
                    range: Some(span_to_range(&task.name_span)),
                }));
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        // Check if cursor is on an include path
        if let Some(inc_path) = self.find_include_path_at_position(uri, pos) {
            // Resolve relative to the current file's directory
            if let Ok(file_path) = uri.to_file_path() {
                let dir = file_path.parent().unwrap_or(&file_path);
                let target = dir.join(&inc_path);
                if let Ok(target_uri) = Url::from_file_path(&target) {
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: target_uri,
                        range: Range::default(),
                    })));
                }
            }
        }

        // Check if cursor is on a task name (in depends context or elsewhere)
        let word = match self.find_word_at_position(uri, pos) {
            Some(w) => w,
            None => return Ok(None),
        };

        let ast = match self.get_ast(uri) {
            Some(a) => a,
            None => return Ok(None),
        };

        for task in &ast.tasks {
            if task.name == word {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: uri.clone(),
                    range: span_to_range(&task.name_span),
                })));
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let ast = match self.get_ast(uri) {
            Some(a) => a,
            None => return Ok(None),
        };

        #[allow(deprecated)]
        let mut symbols: Vec<SymbolInformation> = Vec::new();

        for task in &ast.tasks {
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: task.name.clone(),
                kind: SymbolKind::FUNCTION,
                location: Location {
                    uri: uri.clone(),
                    range: span_to_range(&task.span),
                },
                tags: None,
                deprecated: None,
                container_name: None,
            });
        }

        for export in &ast.exports {
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: export.key.clone(),
                kind: SymbolKind::VARIABLE,
                location: Location {
                    uri: uri.clone(),
                    range: span_to_range(&export.span),
                },
                tags: None,
                deprecated: None,
                container_name: None,
            });
        }

        for alias in &ast.aliases {
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: alias.name.clone(),
                kind: SymbolKind::FUNCTION,
                location: Location {
                    uri: uri.clone(),
                    range: span_to_range(&alias.span),
                },
                tags: None,
                deprecated: None,
                container_name: None,
            });
        }

        for include in &ast.includes {
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: include.path.clone(),
                kind: SymbolKind::MODULE,
                location: Location {
                    uri: uri.clone(),
                    range: span_to_range(&include.span),
                },
                tags: None,
                deprecated: None,
                container_name: None,
            });
        }

        for dotenv in &ast.dotenv {
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: dotenv.path.clone(),
                kind: SymbolKind::FILE,
                location: Location {
                    uri: uri.clone(),
                    range: span_to_range(&dotenv.span),
                },
                tags: None,
                deprecated: None,
                container_name: None,
            });
        }

        Ok(Some(DocumentSymbolResponse::Flat(symbols)))
    }
}
