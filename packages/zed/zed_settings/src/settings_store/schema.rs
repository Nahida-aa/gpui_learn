use collections::HashMap;

use super::SettingsStore;
use crate::{FileTypeMap, ThemeName};
use gpui::SharedString;
use schemars::{JsonSchema, json_schema};
use serde_json::Value;
use settings_content::{
    CommandAliasTarget, ExtendingSet, FontFamilyName, IconThemeName, LanguageSettingsContent,
    LanguageToSettingsMap, LspSettings, LspSettingsMap, ProjectSettingsContent,
    UserSettingsContent,
};
use util::schemars::{AllowTrailingCommas, DefaultDenyUnknownFields, replace_subschema};

pub struct SettingsJsonSchemaParams<'a> {
    pub language_names: &'a [String],
    pub font_names: &'a [String],
    pub theme_names: &'a [SharedString],
    pub icon_theme_names: &'a [SharedString],
    pub lsp_adapter_names: &'a [String],
    pub action_names: &'a [&'a str],
    pub action_documentation: &'a HashMap<&'a str, &'a str>,
    pub deprecations: &'a HashMap<&'a str, &'a str>,
    pub deprecation_messages: &'a HashMap<&'a str, &'a str>,
}

pub const LSP_SETTINGS_SCHEMA_URL_PREFIX: &str = "zed://schemas/settings/lsp/";

impl SettingsStore {
    fn configure_schema_generator(
        generator: &mut schemars::SchemaGenerator,
        params: &SettingsJsonSchemaParams,
    ) {
        let language_settings_content_ref = generator
            .subschema_for::<LanguageSettingsContent>()
            .to_value();

        if !params.language_names.is_empty() {
            replace_subschema::<LanguageToSettingsMap>(generator, || {
                json_schema!({
                    "type": "object",
                    "errorMessage": "No language with this name is installed.",
                    "properties": params.language_names.iter().map(|name| (name.clone(), language_settings_content_ref.clone())).collect::<serde_json::Map<_, _>>()
                })
            });

            let file_type_patterns_ref =
                generator.subschema_for::<ExtendingSet<String>>().to_value();
            replace_subschema::<FileTypeMap>(generator, || {
                json_schema!({
                    "type": "object",
                    "errorMessage": "No language with this name is installed.",
                    "properties": params.language_names.iter().map(|name| (name.clone(), file_type_patterns_ref.clone())).collect::<serde_json::Map<_, _>>(),
                    "additionalProperties": file_type_patterns_ref.clone()
                })
            });
        }

        generator.subschema_for::<LspSettings>();

        let lsp_settings_definition = generator
            .definitions()
            .get("LspSettings")
            .expect("LspSettings should be defined")
            .clone();

        if !params.lsp_adapter_names.is_empty() {
            replace_subschema::<LspSettingsMap>(generator, || {
                let mut lsp_properties = serde_json::Map::new();

                for adapter_name in params.lsp_adapter_names {
                    let mut base_lsp_settings = lsp_settings_definition
                        .as_object()
                        .expect("LspSettings should be an object")
                        .clone();

                    if let Some(properties) = base_lsp_settings.get_mut("properties") {
                        if let Some(properties_object) = properties.as_object_mut() {
                            properties_object.insert(
                            "initialization_options".to_string(),
                            serde_json::json!({
                                "$ref": format!("{LSP_SETTINGS_SCHEMA_URL_PREFIX}{adapter_name}/initialization_options")
                            }),
                        );
                            properties_object.insert(
                            "settings".to_string(),
                            serde_json::json!({
                                "$ref": format!("{LSP_SETTINGS_SCHEMA_URL_PREFIX}{adapter_name}/settings")
                            }),
                        );
                        }
                    }

                    lsp_properties.insert(
                        adapter_name.clone(),
                        serde_json::Value::Object(base_lsp_settings),
                    );
                }

                json_schema!({
                    "type": "object",
                    "properties": lsp_properties
                })
            });
        }
    }

    pub fn json_schema(params: &SettingsJsonSchemaParams) -> Value {
        let mut generator = schemars::generate::SchemaSettings::draft2019_09()
            .with_transform(DefaultDenyUnknownFields)
            .with_transform(AllowTrailingCommas)
            .into_generator();

        UserSettingsContent::json_schema(&mut generator);
        Self::configure_schema_generator(&mut generator, params);

        if !params.font_names.is_empty() {
            replace_subschema::<FontFamilyName>(&mut generator, || {
                json_schema!({
                     "type": "string",
                     "enum": params.font_names,
                })
            });
        }

        if !params.theme_names.is_empty() {
            replace_subschema::<ThemeName>(&mut generator, || {
                json_schema!({
                    "type": "string",
                    "enum": params.theme_names,
                })
            });
        }

        if !params.icon_theme_names.is_empty() {
            replace_subschema::<IconThemeName>(&mut generator, || {
                json_schema!({
                    "type": "string",
                    "enum": params.icon_theme_names,
                })
            });
        }

        if !params.action_names.is_empty() {
            replace_subschema::<CommandAliasTarget>(&mut generator, || {
                CommandAliasTarget::build_schema(
                    params.action_names,
                    params.action_documentation,
                    params.deprecations,
                    params.deprecation_messages,
                )
            });
        }

        generator
            .root_schema_for::<UserSettingsContent>()
            .to_value()
    }

    pub fn project_json_schema(params: &SettingsJsonSchemaParams) -> Value {
        let mut generator = schemars::generate::SchemaSettings::draft2019_09()
            .with_transform(DefaultDenyUnknownFields)
            .with_transform(AllowTrailingCommas)
            .into_generator();

        ProjectSettingsContent::json_schema(&mut generator);
        Self::configure_schema_generator(&mut generator, params);

        generator
            .root_schema_for::<ProjectSettingsContent>()
            .to_value()
    }
}
