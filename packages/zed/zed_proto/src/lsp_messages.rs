lsp_messages!(
    (GetReferences, GetReferencesResponse, true),
    (GetDocumentColor, GetDocumentColorResponse, true),
    (GetFoldingRanges, GetFoldingRangesResponse, true),
    (GetDocumentSymbols, GetDocumentSymbolsResponse, true),
    (GetDocumentLinks, GetDocumentLinksResponse, true),
    (GetHover, GetHoverResponse, true),
    (GetCodeActions, GetCodeActionsResponse, true),
    (GetSignatureHelp, GetSignatureHelpResponse, true),
    (GetCodeLens, GetCodeLensResponse, true),
    (GetDocumentDiagnostics, GetDocumentDiagnosticsResponse, true),
    (GetDefinition, GetDefinitionResponse, true),
    (
        GetEditPredictionDefinition,
        GetEditPredictionDefinitionResponse,
        true
    ),
    (
        GetEditPredictionTypeDefinition,
        GetEditPredictionTypeDefinitionResponse,
        true
    ),
    (GetDeclaration, GetDeclarationResponse, true),
    (GetTypeDefinition, GetTypeDefinitionResponse, true),
    (GetImplementation, GetImplementationResponse, true),
    (InlayHints, InlayHintsResponse, false),
    (SemanticTokens, SemanticTokensResponse, true),
    (PrepareCallHierarchy, PrepareCallHierarchyResponse, true),
    (GetIncomingCalls, GetIncomingCallsResponse, true),
    (GetOutgoingCalls, GetOutgoingCallsResponse, true),
);
