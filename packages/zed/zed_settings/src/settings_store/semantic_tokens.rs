use settings_content::SemanticTokenRules;

#[derive(Default)]
pub struct DefaultSemanticTokenRules(pub SemanticTokenRules);

impl gpui::Global for DefaultSemanticTokenRules {}
