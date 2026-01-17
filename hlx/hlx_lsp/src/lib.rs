#![allow(dead_code, unused_variables)]

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
mod builtins;
mod backend_compat;
mod control_flow;
mod dataflow;
mod cfg_builder;
mod type_system;
mod type_inference;
mod quick_fixes;
mod semantic_tokens;
mod module_support;
mod shader_validator;
mod capability_validator;
mod formatter;
mod call_hierarchy;
mod folding_ranges;
mod auto_import;
mod snippets;
mod enhanced_type_inference;
mod ai_context_generation;
mod contract_synthesis;
mod pattern_learning;
mod intent_detection;
mod test_runner;
mod import_organization;
pub mod codegen;

use contracts::{ContractCache, ContractCatalogue};
use ai_diagnostics::AIDiagnosticBuilder;
use patterns::PatternLibrary;
use cfg_builder::CfgBuilder;
use dataflow::{DataflowAnalyzer, UseCertainty};
use type_inference::TypeInference;
use quick_fixes::{QuickFixGenerator, QuickFixContext};
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
use builtins::BuiltinRegistry;
use backend_compat::BackendCompatChecker;
use semantic_tokens::SemanticTokensProvider;
use module_support::ModuleSupport;
use shader_validator::ShaderContractCache;
use capability_validator::CapabilityValidator;
use formatter::HlxFormatter;
use call_hierarchy::CallHierarchyIndex;
use folding_ranges::FoldingRangesProvider;
use auto_import::AutoImportProvider;
use snippets::SnippetProvider;
use enhanced_type_inference::EnhancedTypeInference;
use ai_context_generation::AIContextGenerator;
use contract_synthesis::ContractSynthesizer;
use pattern_learning::PatternLearner;
use intent_detection::IntentDetector;
use test_runner::TestRunnerProvider;
use import_organization::ImportOrganizer;
use tokio::sync::Mutex;

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
    builtin_registry: Arc<BuiltinRegistry>,
    backend_compat: Arc<BackendCompatChecker>,
    quick_fix_generator: Arc<QuickFixGenerator>,
    semantic_tokens_provider: Arc<SemanticTokensProvider>,
    module_support: Arc<Mutex<ModuleSupport>>,
    shader_contract_cache: Arc<ShaderContractCache>,
    capability_validator: Arc<CapabilityValidator>,
    formatter: Arc<HlxFormatter>,
    call_hierarchy_index: Arc<CallHierarchyIndex>,
    folding_ranges_provider: Arc<FoldingRangesProvider>,
    auto_import_provider: Arc<AutoImportProvider>,
    snippet_provider: Arc<tokio::sync::Mutex<SnippetProvider>>,
    enhanced_type_inference: Arc<tokio::sync::Mutex<EnhancedTypeInference>>,
    // Phase 8: AI-Native Features
    ai_context_generator: Arc<AIContextGenerator>,
    contract_synthesizer: Arc<ContractSynthesizer>,
    pattern_learner: Arc<tokio::sync::Mutex<PatternLearner>>,
    intent_detector: Arc<tokio::sync::Mutex<IntentDetector>>,
    // Phase 6: Testing & Validation
    test_runner: Arc<tokio::sync::Mutex<TestRunnerProvider>>,
    // Phase 7: Workspace Intelligence
    import_organizer: Arc<ImportOrganizer>,
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
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: self.semantic_tokens_provider.get_legend(),
                            range: Some(true),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        }
                    )
                ),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
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

        // Index call hierarchy
        self.call_hierarchy_index.index_document(&params.text_document.uri, &params.text_document.text);

        // Update import cache
        self.auto_import_provider.update_imports(&params.text_document.uri, &params.text_document.text);

        self.validate_document(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        // Since we use Full sync, the last item has the full text
        if let Some(event) = params.content_changes.pop() {
            self.document_map.insert(params.text_document.uri.to_string(), event.text.clone());

            // Re-index symbols
            self.symbol_index.index_document(&params.text_document.uri, &event.text);

            // Re-index call hierarchy
            self.call_hierarchy_index.index_document(&params.text_document.uri, &event.text);

            // Update import cache
            self.auto_import_provider.update_imports(&params.text_document.uri, &event.text);

            self.validate_document(params.text_document.uri, event.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Clean up document from map to prevent memory leak
        self.document_map.remove(params.text_document.uri.as_str());

        // Clean up symbols for this document
        self.symbol_index.remove_document(&params.text_document.uri);

        // Clear diagnostics for closed document
        self.client.publish_diagnostics(params.text_document.uri, vec![], None).await;
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            // Try symbol index first (local symbols)
            if let Some(location) = self.symbol_index.find_definition(&position, &uri, &doc) {
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }

            // Try cross-module definitions (imported symbols)
            let word = self.get_word_at_position(&doc, position);
            if let Some(symbol) = word {
                // Find which module this symbol was imported from
                if let Some(module_path) = self.find_import_for_symbol(&doc, &symbol) {
                    let mut module_support = self.module_support.lock().await;
                    if let Some(location) = module_support.find_import_definition(&symbol, &module_path) {
                        return Ok(Some(GotoDefinitionResponse::Scalar(location)));
                    }
                }
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

        // Security: Limit query length to prevent DOS
        const MAX_QUERY_LENGTH: usize = 1000;
        if query.len() > MAX_QUERY_LENGTH {
            return Ok(None);
        }

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

        // Check for import completions
        if let Some(doc_ref) = self.document_map.get(uri.as_str()) {
            let doc = doc_ref.value();

            // Check if we're typing an import path: import "std.|"
            if let Some(partial_path) = self.extract_partial_import_path(doc, position) {
                let support = self.module_support.lock().await;
                items.extend(support.get_import_path_completions(&partial_path));
            }

            // Check if we're typing import symbols: import "std.math" { p| }
            if let Some(module_path) = self.extract_import_module_for_symbols(doc, position) {
                let support = self.module_support.lock().await;
                items.extend(support.get_import_symbol_completions(&module_path));
            }
        }

        // Add context-aware snippets
        if let Some(doc_ref) = self.document_map.get(uri.as_str()) {
            let doc = doc_ref.value();
            let snippet_provider = self.snippet_provider.lock().await;
            let context = snippet_provider.detect_context(doc, position);

            // Get the word being typed (for prefix matching)
            let prefix = self.get_word_at_position_prefix(doc, position);

            let snippet_completions = snippet_provider.get_completions(&prefix, context);
            items.extend(snippet_completions);
        }

        let is_contract_context = if let Some(doc_ref) = self.document_map.get(uri.as_str()) {
            self.is_typing_contract(&doc_ref, position)
        } else {
            false
        };

        // Check for natural language query in comment
        let comment_query = if let Some(doc_ref) = self.document_map.get(uri.as_str()) {
            self.extract_comment_query(&doc_ref, position)
        } else {
            None
        };

        // Natural language search: "// split string" → suggests @305
        if let (Some(query), Some(catalogue), Some(engine)) =
            (comment_query.as_ref(), self.contracts.as_ref(), self.suggestion_engine.as_ref()) {
            let suggestions = engine.suggest(query, catalogue, 5);
            for suggestion in suggestions {
                let contract_id = suggestion.contract_id.trim_start_matches('@');
                if let Some(spec) = catalogue.get_contract(contract_id) {
                    let snippet = catalogue.generate_snippet(contract_id)
                        .unwrap_or_else(|| format!("@{} {{ }}", contract_id));

                    items.push(CompletionItem {
                        label: format!("@{} - {}", contract_id, spec.name),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(format!("Score: {:.0}% - {}", suggestion.score * 100.0, spec.tier)),
                        documentation: Some(Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("{}\n\n**Usage:**\n```hlx\n{}\n```",
                                spec.description, spec.usage),
                        })),
                        insert_text: Some(snippet.clone()),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        sort_text: Some(format!("{:03}", (100.0 - suggestion.score * 100.0) as u32)),
                        filter_text: Some(query.clone()),
                        ..Default::default()
                    });
                }
            }
        }

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

        // Code Snippets
        items.push(CompletionItem {
            label: "game_loop".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Game loop pattern with safe exit".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "Standard game loop pattern with running flag and safe iteration limit".to_string(),
            })),
            insert_text: Some("let running = true;\nloop(running, DEFAULT_MAX_ITER) {\n    // Update game state\n    $1\n\n    // Render\n    $2\n\n    // Check exit condition\n    if ($3) {\n        running = false;\n    }\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });

        // Basic Static Completion (keywords)
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

            // Constants
            ("DEFAULT_MAX_ITER", CompletionItemKind::CONSTANT, "Safety constant (1,000,000)"),
        ];

        for (label, kind, detail) in keywords {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(kind),
                detail: Some(detail.to_string()),
                ..Default::default()
            });
        }

        // P0 Enhancement: Dynamic builtin function completion from registry
        for sig in self.builtin_registry.all() {
            items.push(CompletionItem {
                label: sig.name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(sig.format_signature()),
                documentation: if sig.example.is_some() {
                    Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: sig.format_documentation(),
                    }))
                } else {
                    Some(Documentation::String(sig.description.to_string()))
                },
                insert_text: Some(format!("{}($0)", sig.name)),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(ref doc) = doc_ref {
            let word = self.get_word_at_position(doc, position);
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

                // Check for module/import hover
                if let Some(ref doc_str) = doc_ref {
                    let mut module_support = self.module_support.lock().await;
                    if let Some(hover) = module_support.get_import_hover(&word, position, doc_str) {
                        return Ok(Some(hover));
                    }
                }

                // P0 Enhancement: Check builtin registry first
                if let Some(sig) = self.builtin_registry.get(&word) {
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: sig.format_documentation(),
                        }),
                        range: None,
                    }));
                }

                // Known keywords and constants
                let hover_text = match word.as_str() {
                    "fn" => "## Function Definition\n`fn name(args) { ... }`",
                    "let" => "## Variable Declaration\n`let name = value;`",
                    "if" => "## Conditional\n`if (cond) { ... } else { ... }`",
                    "loop" => "## Loop Construct\n`loop (condition, max_iter) { ... }`\n\n**Note:** Use `DEFAULT_MAX_ITER` for the bound.",
                    "DEFAULT_MAX_ITER" => "## Safety Constant\nValue: `1,000,000`\n\nUse this for all loops to ensure termination.",
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
            // 0. Quick fixes for diagnostics
            let context = &params.context;
            for diagnostic in &context.diagnostics {
                // Check if diagnostic overlaps with requested range
                if self.ranges_overlap(&diagnostic.range, &range) {
                    let ctx = QuickFixContext {
                        uri: uri.clone(),
                        diagnostic: diagnostic.clone(),
                        source_text: doc.to_string(),
                    };

                    let fixes = self.quick_fix_generator.generate_fixes(&ctx);
                    for fix in fixes {
                        all_actions.push(CodeActionOrCommand::CodeAction(fix));
                    }
                }
            }

            // Generate "Fix All" action if there are multiple errors
            if context.diagnostics.len() > 1 {
                if let Some(fix_all) = self.quick_fix_generator.generate_fix_all(&uri, &context.diagnostics, &doc) {
                    all_actions.push(CodeActionOrCommand::CodeAction(fix_all));
                }
            }

            // 0b. Auto-import suggestions for undefined symbols
            for diagnostic in &context.diagnostics {
                if self.ranges_overlap(&diagnostic.range, &range) {
                    // Check if this is an "undefined" error
                    if diagnostic.message.contains("undefined") || diagnostic.message.contains("not found") {
                        // Try to extract the symbol name from the diagnostic message
                        if let Some(symbol) = self.extract_symbol_from_diagnostic(&diagnostic.message) {
                            // Generate auto-import suggestions
                            let import_actions = self.auto_import_provider.generate_code_actions(
                                &symbol,
                                &uri,
                                &doc,
                                range.start,
                            );

                            for action in import_actions {
                                all_actions.push(action);
                            }
                        }
                    }
                }
            }

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

            // P0 Enhancement: Magic number hints for SDL constants
            let magic_number_hints = self.generate_magic_number_hints(&doc, &params.range);
            all_hints.extend(magic_number_hints);
        }

        if !all_hints.is_empty() {
            Ok(Some(all_hints))
        } else {
            Ok(None)
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            let tokens = self.semantic_tokens_provider.provide_semantic_tokens(
                &doc,
                &self.symbol_index,
                &self.builtin_registry,
                &uri,
            );

            match tokens {
                Some(data) => Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                    result_id: None,
                    data,
                }))),
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        let uri = params.text_document.uri;
        let range = params.range;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            let tokens = self.semantic_tokens_provider.provide_semantic_tokens_range(
                &doc,
                range,
                &self.symbol_index,
                &self.builtin_registry,
                &uri,
            );

            match tokens {
                Some(data) => Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
                    result_id: None,
                    data,
                }))),
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            Ok(self.formatter.format_document(&doc))
        } else {
            Ok(None)
        }
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let range = params.range;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            Ok(self.formatter.format_range(&doc, range))
        } else {
            Ok(None)
        }
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<CallHierarchyItem>>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            if let Some(item) = self.call_hierarchy_index.prepare(&uri, position, &doc) {
                Ok(Some(vec![item]))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
        let function_name = &params.item.name;
        let calls = self.call_hierarchy_index.get_incoming_calls(function_name);

        if calls.is_empty() {
            Ok(None)
        } else {
            Ok(Some(calls))
        }
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        let function_name = &params.item.name;
        let calls = self.call_hierarchy_index.get_outgoing_calls(function_name);

        if calls.is_empty() {
            Ok(None)
        } else {
            Ok(Some(calls))
        }
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            Ok(self.folding_ranges_provider.provide_folding_ranges(&doc))
        } else {
            Ok(None)
        }
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        // Try to load contract catalogue with security validation
        let catalogue_path = Self::get_safe_catalogue_path();

        let contracts = match catalogue_path.and_then(|path| {
            Self::validate_catalogue_file(&path)?;
            ContractCache::new(&path).ok()
        }) {
            Some(cache) => {
                eprintln!("✓ Loaded contract catalogue");
                Some(cache.clone_arc())
            }
            None => {
                eprintln!("⚠ Failed to load contract catalogue");
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

        // Create builtin registry
        let builtin_registry = Arc::new(BuiltinRegistry::new());
        eprintln!("✓ Builtin registry ready ({} functions)", builtin_registry.all().count());

        // Create backend compatibility checker
        let backend_compat = Arc::new(BackendCompatChecker::new());
        eprintln!("✓ Backend compatibility checker ready");

        // Create quick fix generator
        let quick_fix_generator = Arc::new(QuickFixGenerator::new());
        eprintln!("✓ Quick fix generator ready");

        // Create semantic tokens provider
        let semantic_tokens_provider = Arc::new(SemanticTokensProvider::new());
        eprintln!("✓ Semantic tokens provider ready");

        // Create module support
        let module_support = Arc::new(Mutex::new(ModuleSupport::new()));
        eprintln!("✓ Module support ready");

        // Create shader contract cache
        let shader_contract_cache = Arc::new(ShaderContractCache::new());
        eprintln!("✓ Shader contract cache ready");

        // Create capability validator
        let capability_validator = Arc::new(CapabilityValidator::new());
        // Try to load capabilities from runtime
        if let Err(e) = capability_validator.load_capabilities() {
            eprintln!("⚠ Failed to load runtime capabilities: {}", e);
            eprintln!("  Builtin validation will be limited");
        } else {
            eprintln!("✓ Runtime capabilities loaded");
        }

        // Create formatter
        let formatter = Arc::new(HlxFormatter::with_default());
        eprintln!("✓ Document formatter ready");

        // Create call hierarchy index
        let call_hierarchy_index = Arc::new(CallHierarchyIndex::new());
        eprintln!("✓ Call hierarchy index ready");

        // Create folding ranges provider
        let folding_ranges_provider = Arc::new(FoldingRangesProvider::new());
        eprintln!("✓ Folding ranges provider ready");

        // Create auto-import provider
        let auto_import_provider = Arc::new(AutoImportProvider::new(symbol_index.clone()));
        eprintln!("✓ Auto-import provider ready");

        // Create snippet provider
        let snippet_provider = Arc::new(tokio::sync::Mutex::new(SnippetProvider::new()));
        eprintln!("✓ Snippet provider ready ({} snippets)", {
            let provider = SnippetProvider::new();
            provider.count()
        });

        // Create enhanced type inference
        let enhanced_type_inference = Arc::new(tokio::sync::Mutex::new(EnhancedTypeInference::new()));
        eprintln!("✓ Enhanced type inference ready");

        // Phase 8: AI-Native Features
        let ai_context_generator = Arc::new(AIContextGenerator::new());
        eprintln!("✓ AI context generator ready");

        let contract_synthesizer = Arc::new(ContractSynthesizer::new());
        eprintln!("✓ Contract synthesizer ready");

        let pattern_learner = Arc::new(tokio::sync::Mutex::new(PatternLearner::new()));
        eprintln!("✓ Pattern learner ready");

        let intent_detector = Arc::new(tokio::sync::Mutex::new(IntentDetector::new()));
        eprintln!("✓ Intent detector ready");

        // Phase 6: Testing & Validation
        let test_runner = Arc::new(tokio::sync::Mutex::new(TestRunnerProvider::new()));
        eprintln!("✓ Test runner ready");

        // Phase 7: Workspace Intelligence
        let import_organizer = Arc::new(ImportOrganizer::new());
        eprintln!("✓ Import organizer ready");

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
            builtin_registry,
            backend_compat,
            quick_fix_generator,
            semantic_tokens_provider,
            module_support,
            shader_contract_cache,
            capability_validator,
            formatter,
            call_hierarchy_index,
            folding_ranges_provider,
            auto_import_provider,
            snippet_provider,
            enhanced_type_inference,
            ai_context_generator,
            contract_synthesizer,
            pattern_learner,
            intent_detector,
            test_runner,
            import_organizer,
        }
    }

    /// Get safe catalogue path with security validation
    fn get_safe_catalogue_path() -> Option<String> {
        // Check environment variable first
        if let Ok(env_path) = std::env::var("HLX_CONTRACT_CATALOGUE") {
            let path = std::path::Path::new(&env_path);

            // Security: Only allow absolute paths
            if !path.is_absolute() {
                eprintln!("⚠ HLX_CONTRACT_CATALOGUE must be an absolute path");
                return None;
            }

            // Security: Validate path doesn't escape to parent directories
            if env_path.contains("..") {
                eprintln!("⚠ HLX_CONTRACT_CATALOGUE cannot contain '..'");
                return None;
            }

            return Some(env_path);
        }

        // Default: Look in common system locations
        let default_paths = [
            "/etc/hlx/CONTRACT_CATALOGUE.json",
            "/usr/local/share/hlx/CONTRACT_CATALOGUE.json",
            "/opt/hlx/CONTRACT_CATALOGUE.json",
        ];

        for path in &default_paths {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }

        // Fallback: Try relative to executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let catalogue = exe_dir.join("CONTRACT_CATALOGUE.json");
                if catalogue.exists() {
                    if let Some(path_str) = catalogue.to_str() {
                        return Some(path_str.to_string());
                    }
                }
            }
        }

        None
    }

    /// Validate catalogue file before loading
    fn validate_catalogue_file(path: &str) -> Option<()> {
        // Check file exists
        let path_obj = std::path::Path::new(path);
        if !path_obj.exists() {
            eprintln!("⚠ Contract catalogue not found: {}", path);
            return None;
        }

        // Security: Check file size limit (10MB max)
        if let Ok(metadata) = std::fs::metadata(path) {
            const MAX_SIZE: u64 = 10_000_000; // 10MB
            if metadata.len() > MAX_SIZE {
                eprintln!("⚠ Contract catalogue too large: {} bytes (max {})",
                         metadata.len(), MAX_SIZE);
                return None;
            }
        } else {
            return None;
        }

        // Security: Validate it's a regular file, not a symlink or device
        if !path_obj.is_file() {
            eprintln!("⚠ Contract catalogue must be a regular file");
            return None;
        }

        Some(())
    }

    async fn validate_document(&self, uri: Url, text: String) {
        // Security: Enforce document size limits to prevent DOS
        const MAX_DOCUMENT_SIZE: usize = 10_000_000; // 10MB
        if text.len() > MAX_DOCUMENT_SIZE {
            let diagnostic = Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 1 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("document-too-large".to_string())),
                source: Some("hlx-security".to_string()),
                message: format!(
                    "Document too large: {} bytes (max {} bytes)\n\n\
                    Large documents can cause performance issues.",
                    text.len(), MAX_DOCUMENT_SIZE
                ),
                related_information: None,
                tags: None,
                code_description: None,
                data: None,
            };
            self.client.publish_diagnostics(uri, vec![diagnostic], None).await;
            return;
        }

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

                    // P0 Enhancement: Add "did you mean?" suggestions for function not found errors
                    let enhanced_message = self.enhance_error_message(&msg);

                    diags.push(Diagnostic {
                        range: Range {
                            start: pos,
                            end: end_pos,
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("hlx".to_string()),
                        message: enhanced_message,
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

        // Add builtin function validation
        // Catch unknown builtins and argument count mismatches
        let builtin_diags = self.validate_builtins(&text);
        diagnostics.extend(builtin_diags);

        // Add builtin availability validation
        // Catch builtins that require missing features or aren't in the runtime
        let availability_diags = self.validate_builtin_availability(&text);
        diagnostics.extend(availability_diags);

        // Add backend compatibility warnings
        // Catch features that work in interpreter but fail in native compilation
        // TODO: Detect target backend from file metadata or config
        // For now, default to checking LLVM compatibility
        let backend_diags = self.backend_compat.check_document(&text, "llvm");
        diagnostics.extend(backend_diags);

        // Add dataflow analysis - detect uninitialized variable uses
        // This catches "Reg type missing" errors before native compilation
        let dataflow_diags = self.check_uninitialized_vars(&text);
        diagnostics.extend(dataflow_diags);

        // Add type checking - detect type mismatches
        // This catches Int vs Float errors, wrong argument types, etc.
        let type_diags = self.check_types(&text);
        diagnostics.extend(type_diags);

        // Add import/module diagnostics
        // Catches missing modules, unused imports, etc.
        let mut module_support = self.module_support.lock().await;
        let module_diags = module_support.validate_imports(&text, &uri);
        diagnostics.extend(module_diags);

        // Add shader contract validation
        // Catches gpu_dispatch mismatches before runtime segfaults
        let shader_diags = self.validate_shader_contracts(&text, &uri);
        diagnostics.extend(shader_diags);

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
                            if let Some(close_brace_rel) = remaining.find('}') {
                                let fields_section = &remaining[..close_brace_rel];

                                // Parse fields with proper position tracking
                                let mut collected_fields = std::collections::HashSet::new();
                                let current_pos = brace_pos + 1;

                                // Split by comma and track positions
                                let mut field_start = 0;
                                for (idx, ch) in fields_section.chars().chain(std::iter::once(',')).enumerate() {
                                    if ch == ',' || idx == fields_section.len() {
                                        let field_pair = &fields_section[field_start..idx];
                                        let field_pair_trimmed = field_pair.trim();

                                        if !field_pair_trimmed.is_empty() {
                                            // Extract field name (before ':')
                                            if let Some(colon_pos_rel) = field_pair_trimmed.find(':') {
                                                let field_name = field_pair_trimmed[..colon_pos_rel].trim();
                                                collected_fields.insert(field_name.to_string());

                                                // Check if this field exists in contract spec
                                                if !spec.fields.contains_key(field_name) {
                                                    // Calculate absolute position
                                                    let trimmed_offset = field_pair.find(field_name).unwrap_or(0);
                                                    let field_offset = current_pos + field_start + trimmed_offset;

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
                                        field_start = idx + 1;
                                    }
                                }

                                // Check for missing required fields
                                for (field_name, field_spec) in &spec.fields {
                                    if field_spec.required && !collected_fields.contains(field_name) {
                                        let pos = Position {
                                            line: line_idx as u32,
                                            character: i as u32,
                                        };

                                        let end_pos = Position {
                                            line: line_idx as u32,
                                            character: (brace_pos + close_brace_rel + 2) as u32,
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

    /// Get the word prefix at the cursor position (for snippet completion)
    fn get_word_at_position_prefix(&self, text: &str, pos: Position) -> String {
        let line = match text.lines().nth(pos.line as usize) {
            Some(l) => l,
            None => return String::new(),
        };

        let char_idx = pos.character as usize;

        if char_idx > line.len() {
            return String::new();
        }

        // Extract word characters before the cursor
        let before = &line[..char_idx];
        let word_start = before
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '@')
            .map(|i| i + 1)
            .unwrap_or(0);

        before[word_start..].to_string()
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

    /// Extract symbol name from diagnostic message
    /// Handles patterns like: "undefined symbol 'foo'", "symbol foo not found", etc.
    fn extract_symbol_from_diagnostic(&self, message: &str) -> Option<String> {
        // Try different patterns for extracting symbol names

        // Pattern: "undefined symbol 'foo'"
        if let Some(start) = message.find("'") {
            if let Some(end) = message[start + 1..].find("'") {
                let symbol = &message[start + 1..start + 1 + end];
                if !symbol.is_empty() {
                    return Some(symbol.to_string());
                }
            }
        }

        // Pattern: "undefined symbol `foo`"
        if let Some(start) = message.find("`") {
            if let Some(end) = message[start + 1..].find("`") {
                let symbol = &message[start + 1..start + 1 + end];
                if !symbol.is_empty() {
                    return Some(symbol.to_string());
                }
            }
        }

        // Pattern: "Undefined function helper"
        if message.contains("Undefined function") {
            let parts: Vec<&str> = message.split_whitespace().collect();
            if let Some(last) = parts.last() {
                return Some(last.to_string());
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

    /// Validate builtin function calls
    /// Catches unknown builtins and argument count mismatches before runtime
    fn validate_builtins(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Regex to match function calls: identifier(args)
        // This is a simple heuristic - doesn't handle all edge cases
        use regex::Regex;
        use std::sync::OnceLock;

        // Pre-compile regex pattern for performance and DOS prevention
        static FUNC_CALL_PATTERN: OnceLock<Regex> = OnceLock::new();
        let func_call_pattern = FUNC_CALL_PATTERN.get_or_init(|| {
            Regex::new(r"(\w+)\s*\(").unwrap()
        });

        for (line_idx, line) in text.lines().enumerate() {
            for cap in func_call_pattern.captures_iter(line) {
                let func_name = cap.get(1).unwrap().as_str();
                let match_start = cap.get(1).unwrap().start();

                // Skip if this looks like a contract invocation (@123) or keyword
                if line[..cap.get(0).unwrap().start()].trim_end().ends_with('@') {
                    continue;
                }

                // Skip common keywords that look like functions
                if ["fn", "loop", "if", "while", "for"].contains(&func_name) {
                    continue;
                }

                // Check if it's a known builtin
                if !self.builtin_registry.exists(func_name) {
                    // Not a builtin - could be user-defined function
                    // Only warn if it looks like a builtin name (lowercase, common patterns)
                    if func_name.chars().all(|c| c.is_lowercase() || c == '_') {
                        // Common builtin-like patterns
                        if ["print", "write", "read", "to_", "parse", "export", "get_"]
                            .iter()
                            .any(|prefix| func_name.starts_with(prefix))
                        {
                            diagnostics.push(Diagnostic {
                                range: Range {
                                    start: Position {
                                        line: line_idx as u32,
                                        character: match_start as u32,
                                    },
                                    end: Position {
                                        line: line_idx as u32,
                                        character: (match_start + func_name.len()) as u32,
                                    },
                                },
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::String("unknown-builtin".to_string())),
                                source: Some("hlx-builtin".to_string()),
                                message: format!(
                                    "Unknown builtin function: {}\nDid you mean one of: {}?",
                                    func_name,
                                    self.suggest_similar_builtin(func_name)
                                ),
                                related_information: None,
                                tags: None,
                                code_description: None,
                                data: None,
                            });
                        }
                    }
                } else {
                    // Known builtin - validate argument count
                    // Count arguments (simple heuristic: commas + 1, or 0 if empty)
                    if let Some(close_paren) = line[cap.get(0).unwrap().end()..].find(')') {
                        let args_str = &line[cap.get(0).unwrap().end()..cap.get(0).unwrap().end() + close_paren];
                        let arg_count = if args_str.trim().is_empty() {
                            0
                        } else {
                            args_str.matches(',').count() + 1
                        };

                        if let Err(msg) = self.builtin_registry.validate_args(func_name, arg_count) {
                            diagnostics.push(Diagnostic {
                                range: Range {
                                    start: Position {
                                        line: line_idx as u32,
                                        character: match_start as u32,
                                    },
                                    end: Position {
                                        line: line_idx as u32,
                                        character: (match_start + func_name.len()) as u32,
                                    },
                                },
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::String("arg-count-mismatch".to_string())),
                                source: Some("hlx-builtin".to_string()),
                                message: msg,
                                related_information: None,
                                tags: None,
                                code_description: None,
                                data: None,
                            });
                        }
                    }
                }
            }
        }

        diagnostics
    }

    /// Validate builtin availability against runtime capabilities
    /// This catches missing builtins and feature mismatches at compile-time
    fn validate_builtin_availability(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only run if capabilities are loaded
        if !self.capability_validator.is_loaded() {
            return diagnostics;
        }

        use regex::Regex;
        use std::sync::OnceLock;

        // Pre-compile regex pattern for performance
        static FUNC_CALL_PATTERN: OnceLock<Regex> = OnceLock::new();
        let func_call_pattern = FUNC_CALL_PATTERN.get_or_init(|| {
            Regex::new(r"(\w+)\s*\(").unwrap()
        });

        for (line_idx, line) in text.lines().enumerate() {
            for cap in func_call_pattern.captures_iter(line) {
                let func_name = cap.get(1).unwrap().as_str();
                let match_start = cap.get(1).unwrap().start();

                // Skip if this looks like a contract invocation (@123) or keyword
                if line[..cap.get(0).unwrap().start()].trim_end().ends_with('@') {
                    continue;
                }

                // Skip common keywords
                if ["fn", "loop", "if", "while", "for"].contains(&func_name) {
                    continue;
                }

                // Only check builtins that exist in the registry
                if !self.builtin_registry.exists(func_name) {
                    continue;
                }

                // Validate against runtime capabilities
                use capability_validator::BuiltinValidation;
                let validation = self.capability_validator.validate_builtin(func_name);

                if let Some(message) = validation.to_diagnostic_message() {
                    let severity = match validation {
                        BuiltinValidation::NotFound { .. } => DiagnosticSeverity::ERROR,
                        BuiltinValidation::MissingFeature { .. } => DiagnosticSeverity::ERROR,
                        _ => DiagnosticSeverity::WARNING,
                    };

                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: match_start as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: (match_start + func_name.len()) as u32,
                            },
                        },
                        severity: Some(severity),
                        code: Some(NumberOrString::String("builtin-unavailable".to_string())),
                        source: Some("hlx-runtime".to_string()),
                        message,
                        related_information: None,
                        tags: None,
                        code_description: None,
                        data: None,
                    });
                }
            }
        }

        diagnostics
    }

    /// Check for uninitialized variable uses via dataflow analysis
    /// This catches "Reg type missing" errors before native compilation
    fn check_uninitialized_vars(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Build CFGs for all functions
        let cfg_builder = CfgBuilder::new();
        let cfgs = cfg_builder.build_all(text);

        // Analyze each function
        for (func_name, cfg) in cfgs {
            let mut analyzer = DataflowAnalyzer::new();
            analyzer.analyze(&cfg);

            // Check for uninitialized uses
            let problems = analyzer.check_all(&cfg);

            for problem in problems {
                let (severity, message) = match problem.certainty {
                    UseCertainty::Definitely => (
                        DiagnosticSeverity::ERROR,
                        format!(
                            "Variable '{}' is used before being initialized.\n\n\
                            In function '{}': This variable is read before any value is assigned.\n\
                            This will cause \"Reg type missing\" errors in native (LLVM) compilation.\n\n\
                            Fix: Initialize '{}' before use, or check your control flow.",
                            problem.var_name, func_name, problem.var_name
                        ),
                    ),
                    UseCertainty::Maybe => (
                        DiagnosticSeverity::WARNING,
                        format!(
                            "Variable '{}' may be uninitialized on some code paths.\n\n\
                            In function '{}': This variable is read, but there exists at least one \
                            execution path where it hasn't been assigned a value.\n\
                            This could cause \"Reg type missing\" errors in native (LLVM) compilation.\n\n\
                            Fix: Ensure '{}' is initialized on all paths before use.",
                            problem.var_name, func_name, problem.var_name
                        ),
                    ),
                };

                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: problem.line as u32,
                            character: 0,
                        },
                        end: Position {
                            line: problem.line as u32,
                            character: 100, // Highlight whole line
                        },
                    },
                    severity: Some(severity),
                    code: Some(NumberOrString::String("uninitialized-variable".to_string())),
                    source: Some("hlx-dataflow".to_string()),
                    message,
                    related_information: None,
                    tags: None,
                    code_description: None,
                    data: None,
                });
            }
        }

        diagnostics
    }

    /// Check for type errors via type inference
    /// This catches Int vs Float mismatches, wrong argument types, etc.
    fn check_types(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Run type inference on the source
        let type_checker = TypeInference::new();
        let errors = type_checker.check_function(text);

        for error in errors {
            let message = match &error.error {
                type_system::TypeError::IncompatibleTypes { op, left, right } => {
                    format!(
                        "Type mismatch in {} operation.\n\n\
                        Cannot apply operator '{}' to types {} and {}.\n\n\
                        Hint: Convert one operand to match the other type.\n\
                        Example: Use 'to_float(x)' to convert Int to Float.",
                        op, op, left, right
                    )
                }
                type_system::TypeError::WrongArgCount { expected, got } => {
                    format!(
                        "Wrong number of arguments.\n\n\
                        Expected {} arguments, but got {}.",
                        expected, got
                    )
                }
                type_system::TypeError::WrongArgType { param_index, expected, got } => {
                    format!(
                        "Type mismatch in argument {}.\n\n\
                        Expected type {}, but got {}.\n\n\
                        Common fixes:\n\
                        - Use 'to_float(x)' to convert Int to Float\n\
                        - Use 'to_int(x)' to convert Float to Int\n\
                        - Use 'to_string(x)' to convert to String",
                        param_index + 1, expected, got
                    )
                }
                type_system::TypeError::UndefinedVariable { name } => {
                    format!(
                        "Undefined variable '{}'.\n\n\
                        This variable is not declared in the current scope.",
                        name
                    )
                }
                type_system::TypeError::UndefinedFunction { name } => {
                    format!(
                        "Undefined function '{}'.\n\n\
                        This function is not a builtin or user-defined function.\n\
                        Check for typos or missing function definitions.",
                        name
                    )
                }
                type_system::TypeError::InvalidUnaryOp { op, operand } => {
                    format!(
                        "Invalid unary operation.\n\n\
                        Cannot apply operator '{}' to type {}.",
                        op, operand
                    )
                }
                type_system::TypeError::HandleCapabilityMismatch { operation, handle_type, requirement } => {
                    format!(
                        "Handle capability mismatch in '{}'.\n\n\
                        Handle type {} does not satisfy requirement: {}.\n\n\
                        Common fixes:\n\
                        - Use 'stage_to_cpu()' to make GPU tensors readable on CPU\n\
                        - Use 'upload_to_gpu()' to move tensors to GPU memory\n\
                        - Use 'seal()' to transition Write handles to Read",
                        operation, handle_type, requirement
                    )
                }
            };

            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: error.line as u32,
                        character: 0,
                    },
                    end: Position {
                        line: error.line as u32,
                        character: 100,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("type-error".to_string())),
                source: Some("hlx-typecheck".to_string()),
                message,
                related_information: None,
                tags: None,
                code_description: None,
                data: None,
            });
        }

        diagnostics
    }

    /// Suggest similar builtin functions using Levenshtein-like heuristic
    fn suggest_similar_builtin(&self, name: &str) -> String {
        let mut candidates: Vec<_> = self.builtin_registry
            .all()
            .filter(|sig| {
                // Simple similarity: starts with same letter or contains substring
                sig.name.starts_with(&name[0..1]) || sig.name.contains(name) || name.contains(sig.name)
            })
            .map(|sig| sig.name)
            .take(3)
            .collect();

        if candidates.is_empty() {
            candidates = vec!["print", "to_string", "read_file"];
        }

        candidates.join(", ")
    }

    /// Extract natural language query from comment
    /// Detects patterns like: "// split string" or "// need: matrix multiply"
    fn extract_comment_query(&self, text: &str, position: Position) -> Option<String> {
        // Security: Limit text size to prevent DOS on large inputs
        const MAX_TEXT_SIZE: usize = 10_000_000; // 10MB
        if text.len() > MAX_TEXT_SIZE {
            return None;
        }

        let lines: Vec<&str> = text.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let line = lines[position.line as usize];

        // Check if we're in a comment
        if let Some(comment_start) = line.find("//") {
            let comment_text = &line[comment_start + 2..].trim();

            // Look for natural language patterns
            // Examples: "// split string", "// need: gpu add", "// matrix multiply"
            if comment_text.len() >= 3 {
                // Remove common prefixes
                let query = comment_text
                    .trim_start_matches("need:")
                    .trim_start_matches("todo:")
                    .trim_start_matches("fixme:")
                    .trim();

                // Only return if it looks like a query (has spaces or specific keywords)
                if query.contains(' ') || query.len() > 5 {
                    return Some(query.to_string());
                }
            }
        }

        None
    }

    /// Extract partial import path for completions
    /// Matches: import "std.|"
    fn extract_partial_import_path(&self, doc: &str, position: Position) -> Option<String> {
        let line = doc.lines().nth(position.line as usize)?;

        // Check if we're in an import statement
        if !line.contains("import") {
            return None;
        }

        // Find the string being typed
        if let Some(quote_start) = line.rfind('"') {
            if quote_start < position.character as usize {
                // We're inside the string
                let partial = &line[quote_start + 1..position.character as usize];
                return Some(partial.to_string());
            }
        }

        None
    }

    /// Extract module path when typing import symbols
    /// Matches: import "std.math" { p| }
    fn extract_import_module_for_symbols(&self, doc: &str, position: Position) -> Option<String> {
        let line = doc.lines().nth(position.line as usize)?;

        // Check if we're in an import statement with braces
        if !line.contains("import") || !line.contains("{") {
            return None;
        }

        // Extract the module path from the string
        if let Some(first_quote) = line.find('"') {
            if let Some(second_quote) = line[first_quote + 1..].find('"') {
                let module_path = &line[first_quote + 1..first_quote + 1 + second_quote];

                // Check if we're after the opening brace
                if let Some(brace_pos) = line.find('{') {
                    if position.character as usize > brace_pos {
                        return Some(module_path.to_string());
                    }
                }
            }
        }

        None
    }

    /// Find which module a symbol was imported from
    /// Searches through import statements to find the source module
    fn find_import_for_symbol(&self, doc: &str, symbol: &str) -> Option<String> {
        // Parse imports from document
        let parser = HlxaParser::new();
        if let Ok(program) = parser.parse_diagnostics(doc) {
            for import in &program.imports {
                // Check if this import includes our symbol
                if let Some(items) = &import.items {
                    for item in items {
                        if item.name == symbol {
                            return Some(import.path.clone());
                        }
                    }
                } else {
                    // Wildcard import - this symbol could be from here
                    // We'd need to resolve the module to check
                    // For now, just return this as a candidate
                    return Some(import.path.clone());
                }
            }
        }

        None
    }

    /// Validate shader contracts for gpu_dispatch calls
    /// This catches shader mismatches before runtime segfaults
    fn validate_shader_contracts(&self, text: &str, uri: &Url) -> Vec<Diagnostic> {
        use regex::Regex;
        use shader_validator::ContractViolation;

        let mut diagnostics = Vec::new();

        // Regex to find gpu_dispatch calls
        // Pattern: gpu_dispatch("shader.spv", [bindings...], push_constants)
        let re = Regex::new(r#"gpu_dispatch\s*\(\s*"([^"]+)""#).unwrap();

        // Get workspace root from URI
        let workspace_root = uri.to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()));

        for (line_idx, line) in text.lines().enumerate() {
            for cap in re.captures_iter(line) {
                let shader_path_str = cap.get(1).unwrap().as_str();
                let match_start = cap.get(0).unwrap().start();

                // Resolve shader path relative to workspace
                let shader_path = if let Some(ref root) = workspace_root {
                    root.join(shader_path_str)
                } else {
                    std::path::PathBuf::from(shader_path_str)
                };

                // Try to load shader contract
                match self.shader_contract_cache.get_or_load(&shader_path) {
                    Ok(contract) => {
                        // Successfully loaded shader contract
                        // Now we need to extract binding count and push constant size from the HLX code
                        // This is a simplified heuristic - in a real implementation, we'd parse the AST

                        // Try to find the bindings array after the shader path
                        // Look for pattern like: [buf1, buf2, buf3]
                        let rest_of_line = &line[cap.get(0).unwrap().end()..];
                        let binding_count = if let Some(arr_start) = rest_of_line.find('[') {
                            if let Some(arr_end) = rest_of_line[arr_start..].find(']') {
                                let arr_content = &rest_of_line[arr_start + 1..arr_start + arr_end];
                                if arr_content.trim().is_empty() {
                                    0
                                } else {
                                    arr_content.matches(',').count() + 1
                                }
                            } else {
                                0
                            }
                        } else {
                            0
                        };

                        // TODO: Extract push constant size
                        // For now, we'll just validate binding count
                        let violations = contract.validate_dispatch(binding_count, None);

                        for violation in violations {
                            let (severity, code) = match &violation {
                                ContractViolation::ShaderNotFound { .. } => {
                                    (DiagnosticSeverity::ERROR, "shader-not-found")
                                }
                                ContractViolation::BindingCountMismatch { .. } => {
                                    (DiagnosticSeverity::ERROR, "binding-count-mismatch")
                                }
                                ContractViolation::PushConstantSizeMismatch { .. } => {
                                    (DiagnosticSeverity::ERROR, "push-constant-mismatch")
                                }
                                ContractViolation::MisalignedPushConstants { .. } => {
                                    (DiagnosticSeverity::ERROR, "misaligned-push-constants")
                                }
                                ContractViolation::InvalidSpirv { .. } => {
                                    (DiagnosticSeverity::ERROR, "invalid-spirv")
                                }
                            };

                            diagnostics.push(Diagnostic {
                                range: Range {
                                    start: Position {
                                        line: line_idx as u32,
                                        character: match_start as u32,
                                    },
                                    end: Position {
                                        line: line_idx as u32,
                                        character: (match_start + cap.get(0).unwrap().as_str().len()) as u32,
                                    },
                                },
                                severity: Some(severity),
                                code: Some(NumberOrString::String(code.to_string())),
                                source: Some("hlx-shader".to_string()),
                                message: format!("{}", violation),
                                related_information: None,
                                tags: None,
                                code_description: None,
                                data: None,
                            });
                        }
                    }
                    Err(e) => {
                        // Shader file not found or invalid SPIR-V
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: line_idx as u32,
                                    character: match_start as u32,
                                },
                                end: Position {
                                    line: line_idx as u32,
                                    character: (match_start + cap.get(0).unwrap().as_str().len()) as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: Some(NumberOrString::String("shader-load-failed".to_string())),
                            source: Some("hlx-shader".to_string()),
                            message: format!("Could not load shader contract: {}", e),
                            related_information: None,
                            tags: None,
                            code_description: None,
                            data: None,
                        });
                    }
                }
            }
        }

        diagnostics
    }

    /// P0 Enhancement: Generate inlay hints for magic numbers (SDL constants, key codes)
    fn generate_magic_number_hints(&self, doc: &str, range: &Range) -> Vec<InlayHint> {
        let mut hints = Vec::new();

        // SDL constant registry
        let sdl_constants: HashMap<i64, &str> = [
            // SDL Events
            (256, "SDL_QUIT"),
            (768, "SDL_KEYDOWN"),
            (769, "SDL_KEYUP"),
            (770, "SDL_TEXTEDITING"),
            (771, "SDL_TEXTINPUT"),
            (1024, "SDL_MOUSEMOTION"),
            (1025, "SDL_MOUSEBUTTONDOWN"),
            (1026, "SDL_MOUSEBUTTONUP"),
            (1027, "SDL_MOUSEWHEEL"),

            // SDL Key Codes (common)
            (8, "BACKSPACE"),
            (9, "TAB"),
            (13, "RETURN"),
            (27, "ESCAPE"),
            (32, "SPACE"),
            (127, "DELETE"),
            (1073741881, "SDLK_CAPSLOCK"),
            (1073741882, "SDLK_F1"),
            (1073741883, "SDLK_F2"),
            (1073741884, "SDLK_F3"),
            (1073741885, "SDLK_F4"),
            (1073741886, "SDLK_F5"),
            (1073741887, "SDLK_F6"),
            (1073741888, "SDLK_F7"),
            (1073741889, "SDLK_F8"),
            (1073741890, "SDLK_F9"),
            (1073741891, "SDLK_F10"),
            (1073741892, "SDLK_F11"),
            (1073741893, "SDLK_F12"),
            (1073741903, "SDLK_RIGHT"),
            (1073741904, "SDLK_LEFT"),
            (1073741905, "SDLK_DOWN"),
            (1073741906, "SDLK_UP"),
            (1073741907, "SDLK_NUMLOCK"),
            (1073741920, "SDLK_LCTRL"),
            (1073741921, "SDLK_LSHIFT"),
            (1073741922, "SDLK_LALT"),
            (1073741923, "SDLK_LGUI"),
            (1073741924, "SDLK_RCTRL"),
            (1073741925, "SDLK_RSHIFT"),
            (1073741926, "SDLK_RALT"),
        ].iter().cloned().collect();

        // Parse document for integer literals
        let lines: Vec<&str> = doc.lines().collect();
        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx as u32;

            // Skip if line is outside requested range
            if line_num < range.start.line || line_num > range.end.line {
                continue;
            }

            // Simple regex-like parsing for integer literals
            let mut chars = line.char_indices().peekable();
            while let Some((idx, ch)) = chars.next() {
                if ch.is_ascii_digit() {
                    // Found start of number
                    let start_idx = idx;
                    let mut num_str = String::from(ch);

                    // Collect all digits
                    while let Some(&(_, next_ch)) = chars.peek() {
                        if next_ch.is_ascii_digit() {
                            num_str.push(next_ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    // Try to parse as integer
                    if let Ok(value) = num_str.parse::<i64>() {
                        // Check if it's a known constant
                        if let Some(const_name) = sdl_constants.get(&value) {
                            hints.push(InlayHint {
                                position: Position {
                                    line: line_num,
                                    character: (start_idx + num_str.len()) as u32,
                                },
                                label: InlayHintLabel::String(format!(" /* {} */", const_name)),
                                kind: Some(InlayHintKind::TYPE),
                                text_edits: None,
                                tooltip: Some(InlayHintTooltip::String(format!(
                                    "Known constant: {}\nValue: {}",
                                    const_name, value
                                ))),
                                padding_left: Some(true),
                                padding_right: Some(false),
                                data: None,
                            });
                        }
                    }
                }
            }
        }

        hints
    }

    /// P0 Enhancement: Enhance error messages with suggestions
    fn enhance_error_message(&self, msg: &str) -> String {
        // Check if this is a "function not found" error
        // Common patterns: "Unknown function: foo", "Undefined function 'foo'", etc.
        if let Some(func_name) = self.extract_function_name_from_error(msg) {
            // Try to find similar functions
            let similar = self.builtin_registry.find_similar(&func_name, 3);

            if !similar.is_empty() {
                // Add suggestions
                let mut enhanced = msg.to_string();
                enhanced.push_str("\n\nDid you mean: ");
                enhanced.push_str(&similar.join(", "));
                enhanced.push_str("?");

                // Show related functions
                let category_matches = self.builtin_registry.get_by_category(&func_name);
                if !category_matches.is_empty() && category_matches.len() <= 5 {
                    enhanced.push_str("\n\nRelated functions:\n");
                    for sig in category_matches.iter().take(5) {
                        enhanced.push_str(&format!("  • {} - {}\n", sig.name, sig.description));
                    }
                }

                return enhanced;
            }
        }

        // No enhancement needed, return original
        msg.to_string()
    }

    /// Extract function name from error message
    fn extract_function_name_from_error(&self, msg: &str) -> Option<String> {
        // Try to extract function name from various error formats
        // "Unknown function: foo" → "foo"
        // "Undefined function 'foo'" → "foo"
        // "Function 'foo' not found" → "foo"

        if let Some(idx) = msg.find("function") {
            let after = &msg[idx..];
            // Look for quoted name
            if let Some(start) = after.find('\'') {
                if let Some(end) = after[start + 1..].find('\'') {
                    return Some(after[start + 1..start + 1 + end].to_string());
                }
            }
            // Look for name after colon
            if let Some(colon) = after.find(':') {
                let name = after[colon + 1..].trim().split_whitespace().next()?;
                return Some(name.to_string());
            }
        }

        None
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