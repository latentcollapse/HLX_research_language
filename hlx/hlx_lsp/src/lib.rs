use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use hlx_compiler::HlxaParser;
use dashmap::DashMap;
use std::sync::Arc;
use std::collections::HashMap;

mod contracts;
mod ai_diagnostics;
mod patterns;
mod confidence;
mod contract_suggestions;
mod auto_correct;
mod inline_preview;
mod state_viz;
mod semantic_diff;
mod constrained_grammar;
mod symbol_index;
mod signature_help;
mod refactoring;
mod performance_lens;
mod code_lens;
mod type_lens;
mod rust_diagnostics;

use contracts::{ContractCache, ContractCatalogue};
use ai_diagnostics::AIDiagnosticBuilder;
use patterns::PatternLibrary;
use confidence::ConfidenceAnalyzer;
use contract_suggestions::ContractSuggestionEngine;
use auto_correct::AutoCorrector;
use inline_preview::InlinePreviewEngine;
use state_viz::StateVizEngine;
use semantic_diff::SemanticDiffAnalyzer;
use constrained_grammar::ConstrainedGrammarValidator;
use symbol_index::SymbolIndex;
use signature_help::{SignatureHelpProvider, SignatureContext};
use refactoring::RefactoringEngine;
use performance_lens::PerformanceLens;
use code_lens::CodeLensProvider;
use type_lens::TypeLens;
use rust_diagnostics::RustDiagnostics;

pub struct Backend {
    client: Client,
    document_map: DashMap<String, String>,
    contracts: Option<Arc<ContractCatalogue>>,
    patterns: Arc<PatternLibrary>,
    suggestion_engine: Option<Arc<ContractSuggestionEngine>>,
    auto_corrector: Arc<AutoCorrector>,
    preview_engine: Arc<InlinePreviewEngine>,
    state_viz_engine: Arc<StateVizEngine>,
    semantic_diff: Arc<SemanticDiffAnalyzer>,
    grammar_validator: Arc<ConstrainedGrammarValidator>,
    symbol_index: Arc<SymbolIndex>,
    signature_help_provider: Arc<SignatureHelpProvider>,
    refactoring_engine: Arc<RefactoringEngine>,
    performance_lens: Arc<PerformanceLens>,
    code_lens_provider: Arc<CodeLensProvider>,
    type_lens: Arc<TypeLens>,
    rust_diagnostics: Arc<RustDiagnostics>,
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
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), ":".to_string(), "@".to_string()]) ,
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["{".to_string(), ",".to_string(), "(".to_string()]),
                    retrigger_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "HLX Language Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.document_map.insert(params.text_document.uri.to_string(), params.text_document.text.clone());

        // Index symbols
        self.symbol_index.index_document(&params.text_document.uri, &params.text_document.text);

        self.validate_document(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        // Since we use Full sync, the last item has the full text
        if let Some(event) = params.content_changes.pop() {
            self.document_map.insert(params.text_document.uri.to_string(), event.text.clone());

            // Re-index symbols
            self.symbol_index.index_document(&params.text_document.uri, &event.text);

            self.validate_document(params.text_document.uri, event.text).await;
        }
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            if let Some(location) = self.symbol_index.find_definition(&position, &uri, &doc) {
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            let refs = self.symbol_index.find_references(&position, &uri, &doc);
            if !refs.is_empty() {
                return Ok(Some(refs));
            }
        }

        Ok(None)
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        let symbols = self.symbol_index.get_document_symbols(&uri);
        if !symbols.is_empty() {
            return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
        }

        Ok(None)
    }

    async fn symbol(&self, params: WorkspaceSymbolParams) -> Result<Option<Vec<SymbolInformation>>> {
        let query = params.query;

        let symbols = self.symbol_index.search_symbols(&query);
        if !symbols.is_empty() {
            return Ok(Some(symbols));
        }

        Ok(None)
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            // Detect context (contract or function)
            let context = self.signature_help_provider.detect_context(&doc, &position);

            match context {
                SignatureContext::Contract => {
                    // Show contract field signatures
                    if let Some(catalogue) = &self.contracts {
                        return Ok(self.signature_help_provider.get_contract_signature(&doc, &position, catalogue));
                    }
                }
                SignatureContext::Function(func_name) => {
                    // Show function parameter signatures
                    return Ok(self.signature_help_provider.get_function_signature(&doc, &position, &func_name));
                }
                SignatureContext::None => {}
            }
        }

        Ok(None)
    }

    async fn prepare_rename(&self, params: TextDocumentPositionParams) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            // Find the symbol at this position
            if let Some(definition) = self.symbol_index.find_definition(&position, &uri, &doc) {
                // Return the range of the symbol
                return Ok(Some(PrepareRenameResponse::Range(definition.range)));
            }
        }

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            return Ok(self.refactoring_engine.rename_symbol(&uri, &position, &new_name, &doc));
        }

        Ok(None)
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            let lenses = self.code_lens_provider.get_code_lenses(&uri, &doc);
            if !lenses.is_empty() {
                return Ok(Some(lenses));
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut items = Vec::new();

        // Check if we're typing a contract (@ID)
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let is_contract_context = if let Some(doc_ref) = self.document_map.get(uri.as_str()) {
            self.is_typing_contract(&doc_ref, position)
        } else {
            false
        };

        if is_contract_context {
            // Show contract completions with smart snippets and context filtering
            if let Some(catalogue) = &self.contracts {
                // Determine what context we're in
                let context = if let Some(doc_ref) = self.document_map.get(uri.as_str()) {
                    self.get_contract_context(&doc_ref, position)
                } else {
                    ContractContext::General
                };

                // Map context to filter string
                let filter_key = match context {
                    ContractContext::MathExpression => "math",
                    ContractContext::ValueExpression => "value",
                    ContractContext::ControlFlow => "control",
                    ContractContext::IOOperation => "io",
                    ContractContext::FieldValue => "field",
                    ContractContext::General => "general",
                };

                // Get filtered contract IDs based on context
                let filtered_ids = catalogue.filter_by_relevance(filter_key);

                for id in filtered_ids {
                    if let Some(spec) = catalogue.get_contract(&id) {
                        let label = format!("@{}", id);
                        let snippet = catalogue.generate_snippet(&id)
                            .unwrap_or_else(|| format!("@{} {{ }}", id));

                        items.push(CompletionItem {
                            label: label.clone(),
                            kind: Some(CompletionItemKind::STRUCT),
                            detail: Some(format!("{} - {}", spec.name, spec.tier)),
                            documentation: Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: spec.description.clone(),
                            })),
                            insert_text: Some(snippet),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        // Basic Static Completion (keywords, builtins)
        let keywords = vec![
            // Keywords
            ("fn", CompletionItemKind::KEYWORD, "Function definition"),
            ("let", CompletionItemKind::KEYWORD, "Variable declaration"),
            ("if", CompletionItemKind::KEYWORD, "Conditional"),
            ("else", CompletionItemKind::KEYWORD, "Alternative"),
            ("loop", CompletionItemKind::KEYWORD, "Loop construct"),
            ("return", CompletionItemKind::KEYWORD, "Return value"),
            ("program", CompletionItemKind::KEYWORD, "Program definition"),
            ("break", CompletionItemKind::KEYWORD, "Break loop"),
            ("continue", CompletionItemKind::KEYWORD, "Continue loop"),
            ("true", CompletionItemKind::KEYWORD, "Boolean true"),
            ("false", CompletionItemKind::KEYWORD, "Boolean false"),
            ("null", CompletionItemKind::KEYWORD, "Null value"),

            // Builtins & Constants
            ("DEFAULT_MAX_ITER", CompletionItemKind::CONSTANT, "Safety constant (1,000,000)"),
            ("print", CompletionItemKind::FUNCTION, "Print value(s)"),
            ("len", CompletionItemKind::FUNCTION, "Get length of array/string"),
            ("to_int", CompletionItemKind::FUNCTION, "Convert to integer"),
            ("slice", CompletionItemKind::FUNCTION, "Slice array"),
            ("append", CompletionItemKind::FUNCTION, "Append to array"),
            ("type", CompletionItemKind::FUNCTION, "Get value type"),
            ("read_file", CompletionItemKind::FUNCTION, "Read file content"),
            ("http_request", CompletionItemKind::FUNCTION, "Make HTTP request"),
            ("json_parse", CompletionItemKind::FUNCTION, "Parse JSON string"),
        ];

        for (label, kind, detail) in keywords {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(kind),
                detail: Some(detail.to_string()),
                ..Default::default()
            });
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            let word = self.get_word_at_position(&doc, position);
            if let Some(word) = word {
                // Check if it's a contract (@ID)
                if word.starts_with('@') {
                    let contract_id = word.trim_start_matches('@');
                    if let Some(catalogue) = &self.contracts {
                        if let Some(hover_doc) = catalogue.format_hover_doc(contract_id) {
                            return Ok(Some(Hover {
                                contents: HoverContents::Markup(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: hover_doc,
                                }),
                                range: None,
                            }));
                        }
                    }
                }

                // Known symbols
                let hover_text = match word.as_str() {
                    "fn" => "## Function Definition\n`fn name(args) { ... }`",
                    "let" => "## Variable Declaration\n`let name = value;`",
                    "if" => "## Conditional\n`if (cond) { ... } else { ... }`",
                    "loop" => "## Loop Construct\n`loop (condition, max_iter) { ... }`\n\n**Note:** Use `DEFAULT_MAX_ITER` for the bound.",
                    "DEFAULT_MAX_ITER" => "## Safety Constant\nValue: `1,000,000`\n\nUse this for all loops to ensure termination.",
                    "to_int" => "## Builtin: to_int\n`to_int(value) -> int`\n\nConverts a value to an integer (truncates floats).",
                    "len" => "## Builtin: len\n`len(container) -> int`\n\nReturns the length of a string, array, or object.",
                    "print" => "## Builtin: print\n`print(val)`\n\nPrints a value to stdout.",
                    "append" => "## Builtin: append\n`append(array, item) -> array`\n\nReturns a new array with the item appended.",
                    "slice" => "## Builtin: slice\n`slice(array, start, len) -> array`\n\nReturns a slice of the array.",
                    "type" => "## Builtin: type\n`type(val) -> string`\n\nReturns the type name of the value.",
                    "http_request" => "## Builtin: http_request\n`http_request(method, url, body, headers) -> Response`\n\nMakes an HTTP request. Returns object with status, body, headers.",
                    "json_parse" => "## Builtin: json_parse\n`json_parse(json_string) -> Value`\n\nParses a JSON string into an HLX value.",
                    _ => return Ok(None),
                };

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_text.to_string(),
                    }),
                    range: None,
                }));
            }
        }

        Ok(None)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;
        let mut all_actions = Vec::new();

        // Get the text in the range (usually a line with a comment)
        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            // 1. Check for auto-corrections
            let corrections = self.auto_corrector.analyze_document(&doc);
            for correction in corrections {
                // Check if this correction overlaps with the requested range
                if self.ranges_overlap(&correction.range, &range) {
                    let action = self.auto_corrector.create_code_action(&correction, uri.clone());
                    all_actions.push(CodeActionOrCommand::CodeAction(action));
                }
            }

            // 2. Check for semantic diffs
            let diffs = self.semantic_diff.analyze(&doc);
            for diff in diffs {
                // Check if this diff overlaps with the requested range
                if self.ranges_overlap(&diff.range, &range) {
                    let action = self.semantic_diff.create_code_action(&diff, uri.clone());
                    all_actions.push(CodeActionOrCommand::CodeAction(action));
                }
            }

            // 3. Check for contract suggestions from comments
            let line_text = self.get_line_text(&doc, range.start.line as usize);

            // Check if this line contains a comment
            if let Some(comment_text) = self.extract_comment(&line_text) {
                // Use suggestion engine to find matching contracts
                if let (Some(engine), Some(catalogue)) = (&self.suggestion_engine, &self.contracts) {
                    let suggestions = engine.suggest(&comment_text, catalogue, 3);

                    if !suggestions.is_empty() {
                        for suggestion in suggestions {
                            // Get snippet for this contract
                            if let Some(snippet) = catalogue.generate_snippet(&suggestion.contract_id) {
                                // Create a code action to insert the contract
                                let edit_range = Range {
                                    start: Position {
                                        line: range.end.line,
                                        character: 0,
                                    },
                                    end: Position {
                                        line: range.end.line,
                                        character: 0,
                                    },
                                };

                                let mut changes = HashMap::new();
                                changes.insert(
                                    uri.clone(),
                                    vec![TextEdit {
                                        range: edit_range,
                                        new_text: format!("    {};\n", snippet.replace("$0", "").replace("$1", "_").replace("$2", "_")),
                                    }],
                                );

                                let action = CodeAction {
                                    title: format!("💡 Use {} (@{}) - {}",
                                        suggestion.contract_name,
                                        suggestion.contract_id,
                                        suggestion.description
                                    ),
                                    kind: Some(CodeActionKind::QUICKFIX),
                                    diagnostics: None,
                                    edit: Some(WorkspaceEdit {
                                        changes: Some(changes),
                                        document_changes: None,
                                        change_annotations: None,
                                    }),
                                    command: None,
                                    is_preferred: Some(all_actions.is_empty()), // First one is preferred
                                    disabled: None,
                                    data: None,
                                };

                                all_actions.push(CodeActionOrCommand::CodeAction(action));
                            }
                        }
                    }
                }
            }

            // 4. Refactoring actions
            // Only show refactoring actions if there's a selection (not just cursor)
            if range.start.line != range.end.line || range.start.character != range.end.character {
                // Extract Function - only if selection is non-empty
                if let Some(edits) = self.refactoring_engine.extract_function(
                    &uri,
                    &range,
                    "extracted_function",
                    &doc
                ) {
                    let action = self.refactoring_engine.create_refactor_action(
                        "🔧 Extract Function".to_string(),
                        uri.clone(),
                        edits
                    );
                    all_actions.push(CodeActionOrCommand::CodeAction(action));
                }

                // Convert to Contract - check if selection looks like a binary operation
                if let Some(edit) = self.refactoring_engine.convert_to_contract(&range, &doc) {
                    let action = self.refactoring_engine.create_refactor_action(
                        "⚡ Convert to Contract".to_string(),
                        uri.clone(),
                        vec![edit]
                    );
                    all_actions.push(CodeActionOrCommand::CodeAction(action));
                }
            }

            // Inline Variable - only at cursor position on a variable
            let cursor = Position {
                line: range.start.line,
                character: range.start.character,
            };
            if let Some(edits) = self.refactoring_engine.inline_variable(&uri, &cursor, &doc) {
                let action = self.refactoring_engine.create_refactor_action(
                    "🔧 Inline Variable".to_string(),
                    uri.clone(),
                    edits
                );
                all_actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        if !all_actions.is_empty() {
            Ok(Some(all_actions))
        } else {
            Ok(None)
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let mut all_hints = Vec::new();

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            // 1. Generate inline execution previews
            if let Some(catalogue) = &self.contracts {
                let previews = self.preview_engine.generate_previews(&doc, catalogue);
                let preview_hints: Vec<InlayHint> = previews
                    .iter()
                    .map(|preview| self.preview_engine.create_inlay_hint(preview))
                    .collect();
                all_hints.extend(preview_hints);

                // 4. Generate performance cost hints
                let costs = self.performance_lens.analyze(&doc, catalogue);
                let cost_hints: Vec<InlayHint> = costs
                    .iter()
                    .map(|cost| self.performance_lens.create_inlay_hint(cost))
                    .collect();
                all_hints.extend(cost_hints);
            }

            // 2. Generate state visualization hints
            let snapshots = self.state_viz_engine.analyze_state(&doc);
            let state_hints = self.state_viz_engine.create_inlay_hints(&snapshots);
            all_hints.extend(state_hints);

            // 3. Generate type inference hints
            let type_hints = self.type_lens.infer_types(&doc);
            let type_inlay_hints: Vec<InlayHint> = type_hints
                .iter()
                .map(|hint| self.type_lens.create_inlay_hint(hint))
                .collect();
            all_hints.extend(type_inlay_hints);
        }

        if !all_hints.is_empty() {
            Ok(Some(all_hints))
        } else {
            Ok(None)
        }
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        // Try to load contract catalogue
        let catalogue_path = std::env::var("HLX_CONTRACT_CATALOGUE")
            .unwrap_or_else(|_| "../CONTRACT_CATALOGUE.json".to_string());

        let contracts = match ContractCache::new(&catalogue_path) {
            Ok(cache) => {
                eprintln!("✓ Loaded contract catalogue from {}", catalogue_path);
                Some(cache.clone_arc())
            }
            Err(e) => {
                eprintln!("⚠ Failed to load contract catalogue: {}",

 e);
                eprintln!("  Contracts will not be available in autocomplete/hover");
                None
            }
        };

        // Load pattern library
        let patterns = Arc::new(PatternLibrary::new());
        eprintln!("✓ Loaded {} HLX patterns", patterns.patterns.len());

        // Create suggestion engine if we have contracts
        let suggestion_engine = contracts.as_ref().map(|cat| {
            let engine = ContractSuggestionEngine::new(cat);
            eprintln!("✓ Contract suggestion engine ready");
            Arc::new(engine)
        });

        // Create auto-correction engine
        let auto_corrector = Arc::new(AutoCorrector::new());
        eprintln!("✓ Auto-correction engine ready");

        // Create inline preview engine
        let preview_engine = Arc::new(InlinePreviewEngine::new());
        eprintln!("✓ Inline preview engine ready");

        // Create state visualization engine
        let state_viz_engine = Arc::new(StateVizEngine::new());
        eprintln!("✓ State visualization engine ready");

        // Create semantic diff analyzer
        let semantic_diff = Arc::new(SemanticDiffAnalyzer::new(&patterns));
        eprintln!("✓ Semantic diff analyzer ready");

        // Create constrained grammar validator
        let grammar_validator = Arc::new(ConstrainedGrammarValidator::new(false));
        eprintln!("✓ Constrained grammar validator ready");

        // Create symbol index
        let symbol_index = Arc::new(SymbolIndex::new());
        eprintln!("✓ Symbol index ready");

        // Create signature help provider
        let signature_help_provider = Arc::new(SignatureHelpProvider::new());
        eprintln!("✓ Signature help provider ready");

        // Create refactoring engine
        let refactoring_engine = Arc::new(RefactoringEngine::new(symbol_index.clone()));
        eprintln!("✓ Refactoring engine ready");

        // Create performance lens
        let performance_lens = Arc::new(PerformanceLens::new());
        eprintln!("✓ Performance lens ready");

        // Create code lens provider
        let code_lens_provider = Arc::new(CodeLensProvider::new(symbol_index.clone()));
        eprintln!("✓ Code lens provider ready");

        // Create type lens
        let type_lens = Arc::new(TypeLens::new());
        eprintln!("✓ Type lens ready");

        // Create Rust diagnostics (Stage 4)
        let rust_diagnostics = Arc::new(RustDiagnostics::new());
        eprintln!("✓ Rust diagnostics (Stage 4) ready");

        Self {
            client,
            document_map: DashMap::new(),
            contracts,
            patterns,
            suggestion_engine,
            auto_corrector,
            preview_engine,
            state_viz_engine,
            semantic_diff,
            grammar_validator,
            symbol_index,
            signature_help_provider,
            refactoring_engine,
            performance_lens,
            code_lens_provider,
            type_lens,
            rust_diagnostics,
        }
    }

    async fn validate_document(&self, uri: Url, text: String) {
        let parser = HlxaParser::new();
        let mut diagnostics = match parser.parse_diagnostics(&text) {
            Ok(_) => vec![], // No parser errors
            Err(errors) => {
                let mut diags = Vec::new();
                for (msg, offset) in errors {
                    let pos = self.offset_to_position(&text, offset);

                    // Highlight the word at that position, or just one char
                    let end_pos = Position {
                        line: pos.line,
                        character: pos.character + 1
                    };

                    diags.push(Diagnostic {
                        range: Range {
                            start: pos,
                            end: end_pos,
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("hlx".to_string()),
                        message: msg,
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
                diags
            }
        };

        // Add contract signature validation
        if let Some(catalogue) = &self.contracts {
            let contract_diags = self.validate_contract_signatures(&text, catalogue);
            diagnostics.extend(contract_diags);
        }

        // Add auto-correction diagnostics
        let corrections = self.auto_corrector.analyze_document(&text);
        for correction in corrections {
            let diagnostic = self.auto_corrector.create_diagnostic(&correction);
            diagnostics.push(diagnostic);
        }

        // Add semantic diff diagnostics
        let diffs = self.semantic_diff.analyze(&text);
        for diff in diffs {
            let diagnostic = self.semantic_diff.create_diagnostic(&diff);
            diagnostics.push(diagnostic);
        }

        // Add grammar validation diagnostics
        let violations = self.grammar_validator.validate(&text);
        for violation in violations {
            let diagnostic = self.grammar_validator.create_diagnostic(&violation);
            diagnostics.push(diagnostic);
        }

        // Add performance lens diagnostics (warnings for very slow operations)
        if let Some(catalogue) = &self.contracts {
            let costs = self.performance_lens.analyze(&text, catalogue);
            for cost in costs {
                if let Some(diagnostic) = self.performance_lens.create_diagnostic(&cost) {
                    diagnostics.push(diagnostic);
                }
            }
        }

        // Add Rust-specific diagnostics (Stage 4)
        // Catch compilation errors before rustc sees them
        let rust_diags = self.rust_diagnostics.analyze(&text);
        diagnostics.extend(rust_diags);

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }

    /// Validate contract field signatures
    fn validate_contract_signatures(&self, text: &str, catalogue: &ContractCatalogue) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let ai_diag = AIDiagnosticBuilder::new(catalogue);

        // Regex to find contract invocations: @123 { field: value, ... }
        // This is a simple pattern matcher - not a full parser
        for (line_idx, line) in text.lines().enumerate() {
            let mut chars = line.chars().enumerate().peekable();

            while let Some((i, ch)) = chars.next() {
                if ch == '@' {
                    // Found @ symbol, try to parse contract ID
                    let mut id_str = String::new();

                    // Collect digits
                    while let Some((_, digit_ch)) = chars.peek() {
                        if digit_ch.is_numeric() {
                            id_str.push(*digit_ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    if id_str.is_empty() {
                        continue;
                    }

                    // Skip whitespace
                    while let Some((_, ws_ch)) = chars.peek() {
                        if ws_ch.is_whitespace() {
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    // Check for opening brace
                    if let Some((brace_pos, '{')) = chars.next() {
                        // Found contract invocation, validate fields
                        if let Some(spec) = catalogue.get_contract(&id_str) {
                            // Extract field names until closing brace
                            let remaining = &line[brace_pos + 1..];
                            if let Some(close_brace) = remaining.find('}') {
                                let fields_section = &remaining[..close_brace];

                                // Parse field names (simple split by comma)
                                for field_pair in fields_section.split(',') {
                                    let field_pair = field_pair.trim();
                                    if field_pair.is_empty() {
                                        continue;
                                    }

                                    // Extract field name (before ':')
                                    if let Some(colon_pos) = field_pair.find(':') {
                                        let field_name = field_pair[..colon_pos].trim();

                                        // Check if this field exists in contract spec
                                        if !spec.fields.contains_key(field_name) {
                                            // Calculate position of field name
                                            let field_offset = brace_pos + 1 +
                                                fields_section[..fields_section.find(field_name).unwrap_or(0)].len();

                                            let pos = Position {
                                                line: line_idx as u32,
                                                character: field_offset as u32,
                                            };

                                            let end_pos = Position {
                                                line: line_idx as u32,
                                                character: (field_offset + field_name.len()) as u32,
                                            };

                                            // Use AI-optimized diagnostic
                                            let valid_fields: Vec<String> = spec.fields.keys().cloned().collect();
                                            let ai_diagnostic = ai_diag.unknown_field(
                                                Range { start: pos, end: end_pos },
                                                field_name,
                                                &id_str,
                                                &valid_fields
                                            );
                                            diagnostics.push(ai_diagnostic.to_diagnostic());
                                        }
                                    }
                                }

                                // Check for missing required fields
                                let provided_fields: std::collections::HashSet<String> =
                                    fields_section.split(',')
                                        .filter_map(|pair| {
                                            pair.trim().split(':').next().map(|s| s.trim().to_string())
                                        })
                                        .collect();

                                for (field_name, field_spec) in &spec.fields {
                                    if field_spec.required && !provided_fields.contains(field_name) {
                                        let pos = Position {
                                            line: line_idx as u32,
                                            character: i as u32,
                                        };

                                        let end_pos = Position {
                                            line: line_idx as u32,
                                            character: (brace_pos + close_brace + 2) as u32,
                                        };

                                        // Use AI-optimized diagnostic
                                        let ai_diagnostic = ai_diag.missing_required_field(
                                            Range { start: pos, end: end_pos },
                                            field_name,
                                            &id_str
                                        );
                                        diagnostics.push(ai_diagnostic.to_diagnostic());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        diagnostics
    }

    fn offset_to_position(&self, text: &str, offset: usize) -> Position {
        let mut line = 0;
        let mut last_line_start = 0;
        
        for (i, c) in text.char_indices() {
            if i >= offset {
                break;
            }
            if c == '\n' {
                line += 1;
                last_line_start = i + 1;
            }
        }
        
        let character = if offset >= last_line_start {
            (offset - last_line_start) as u32
        } else {
            0
        };

        Position { line, character }
    }

    fn get_word_at_position(&self, text: &str, pos: Position) -> Option<String> {
        let line = text.lines().nth(pos.line as usize)?;
        let char_idx = pos.character as usize;

        if char_idx >= line.len() {
            return None;
        }

        // Check if we're on an @ symbol (contract start)
        if char_idx > 0 && line.chars().nth(char_idx - 1) == Some('@') {
            // Look ahead for contract ID
            let start = char_idx;
            let end = line[char_idx..]
                .find(|c: char| !c.is_numeric())
                .map(|i| char_idx + i)
                .unwrap_or(line.len());

            if start < end {
                return Some(format!("@{}", &line[start..end]));
            }
        }

        // Check if we're inside a contract ID (@123)
        if line.chars().nth(char_idx) == Some('@') ||
           (char_idx > 0 && line[..char_idx].chars().rev().take_while(|c| c.is_numeric()).count() > 0) {
            // Find the @ symbol
            let at_pos = line[..=char_idx]
                .rfind('@')
                .unwrap_or(char_idx);

            // Find end of contract ID
            let end = line[at_pos+1..]
                .find(|c: char| !c.is_numeric())
                .map(|i| at_pos + 1 + i)
                .unwrap_or(line.len());

            if at_pos + 1 < end {
                return Some(format!("@{}", &line[at_pos+1..end]));
            }
        }

        // Find start of word (regular identifiers)
        let start = line[..char_idx]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        // Find end of word
        let end = line[char_idx..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| char_idx + i)
            .unwrap_or(line.len());

        if start < end {
            Some(line[start..end].to_string())
        } else {
            None
        }
    }

    fn is_typing_contract(&self, text: &str, pos: Position) -> bool {
        if let Some(line) = text.lines().nth(pos.line as usize) {
            let char_idx = pos.character as usize;
            if char_idx > 0 && char_idx <= line.len() {
                // Check if previous character is '@'
                if line.chars().nth(char_idx - 1) == Some('@') {
                    return true;
                }
                // Check if we're typing digits after '@'
                if let Some(at_pos) = line[..char_idx].rfind('@') {
                    let between = &line[at_pos+1..char_idx];
                    if between.chars().all(|c| c.is_numeric()) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Determine contract context for smart filtering
    fn get_contract_context(&self, text: &str, pos: Position) -> ContractContext {
        let line_idx = pos.line as usize;

        // Look at current line and a few lines before for context
        let context_lines: Vec<&str> = text.lines()
            .skip(line_idx.saturating_sub(3))
            .take(4)
            .collect();

        let current_line = context_lines.last().unwrap_or(&"");

        // Check for specific contexts
        if current_line.contains("let ") && current_line.contains("=") {
            // Variable assignment - values and operations
            return ContractContext::ValueExpression;
        }

        if current_line.contains("if ") || current_line.contains("loop ") {
            // Control flow context
            return ContractContext::ControlFlow;
        }

        // Check for mathematical operators nearby
        if current_line.contains("+") || current_line.contains("-") ||
           current_line.contains("*") || current_line.contains("/") {
            return ContractContext::MathExpression;
        }

        // Check if inside function parameters or object fields
        if let Some(last_open) = current_line.rfind('{') {
            if let Some(last_close) = current_line.rfind('}') {
                if last_open > last_close {
                    // Inside braces, likely a field value
                    return ContractContext::FieldValue;
                }
            } else {
                // Open brace but no close - definitely inside
                return ContractContext::FieldValue;
            }
        }

        // Check for print/io context
        if current_line.contains("print") || context_lines.iter().any(|l| l.contains("http_request")) {
            return ContractContext::IOOperation;
        }

        // Default: show all user-facing contracts
        ContractContext::General
    }

    /// Get the text of a specific line
    fn get_line_text(&self, text: &str, line: usize) -> String {
        text.lines().nth(line).unwrap_or("").to_string()
    }

    /// Extract comment text from a line (supports // and /* */ style)
    fn extract_comment(&self, line: &str) -> Option<String> {
        // Single-line comment
        if let Some(idx) = line.find("//") {
            let comment = &line[idx + 2..].trim();
            if !comment.is_empty() {
                return Some(comment.to_string());
            }
        }

        // Multi-line comment (simple case - comment on one line)
        if let Some(start_idx) = line.find("/*") {
            if let Some(end_idx) = line.find("*/") {
                if end_idx > start_idx {
                    let comment = &line[start_idx + 2..end_idx].trim();
                    if !comment.is_empty() {
                        return Some(comment.to_string());
                    }
                }
            }
        }

        None
    }

    /// Check if two ranges overlap
    fn ranges_overlap(&self, a: &Range, b: &Range) -> bool {
        // Ranges overlap if they're on the same line or intersect
        if a.start.line == b.start.line && a.end.line == b.end.line {
            // Same line - check character positions
            !(a.end.character <= b.start.character || b.end.character <= a.start.character)
        } else {
            // Multi-line overlap check
            !(a.end.line < b.start.line || b.end.line < a.start.line)
        }
    }
}

/// Context types for smart contract filtering
#[derive(Debug, Clone, PartialEq)]
enum ContractContext {
    General,           // Show all user-facing contracts
    ValueExpression,   // Variable assignments, return values
    MathExpression,    // Math operations
    ControlFlow,       // if/loop/pattern matching
    FieldValue,        // Inside { } braces
    IOOperation,       // I/O and external calls
}