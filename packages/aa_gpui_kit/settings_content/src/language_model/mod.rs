//! LLM 设置内容（对齐 Zed `crates/settings_content/src/language_model.rs`）。
//!
//! 当前是 aa.json 旧格式的 Config 兼容层 — CLI 入口直接读 aa.json。
//! 后续将迁移到 Zed `AllLanguageModelSettingsContent` 结构
//! （`language_models.anthropic.api_url` / `language_models.ollama.api_key` 等）。

use collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
use std::{path::PathBuf, sync::Arc};

use crate::language_model::{
    anthropic::{AnthropicCompatibleSettingsContent, AnthropicSettingsContent},
    bedrock::AmazonBedrockSettingsContent,
    deepseek::DeepseekSettingsContent,
    google::GoogleSettingsContent,
    llama_cpp::LlamaCppSettingsContent,
    lmstudio::LmStudioSettingsContent,
    mistral::MistralSettingsContent,
    ollama::OllamaSettingsContent,
    open_router::OpenRouterSettingsContent,
    openai::{OpenAiCompatibleSettingsContent, OpenAiSettingsContent},
    opencode::OpenCodeSettingsContent,
    vercel_ai_gateway::VercelAiGatewaySettingsContent,
    x_ai::XAiSettingsContent,
    zed_dot_dev::ZedDotDevSettingsContent,
};
pub mod anthropic;
pub mod bedrock;
pub mod common;
pub use common::OpenAiReasoningEffort;
pub mod deepseek;
pub mod google;
pub mod llama_cpp;
pub mod lmstudio;
pub mod mistral;
pub mod ollama;
pub mod open_router;
pub mod openai;
pub mod opencode;
pub mod vercel_ai_gateway;
pub mod x_ai;
pub mod zed_dot_dev;

#[with_fallible_options]
#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema, MergeFrom)]
pub struct AllLanguageModelSettingsContent {
    pub anthropic: Option<AnthropicSettingsContent>,
    pub anthropic_compatible: Option<HashMap<Arc<str>, AnthropicCompatibleSettingsContent>>,
    pub bedrock: Option<AmazonBedrockSettingsContent>,
    pub deepseek: Option<DeepseekSettingsContent>,
    pub google: Option<GoogleSettingsContent>,
    #[serde(rename = "llama.cpp")]
    pub llama_cpp: Option<LlamaCppSettingsContent>,
    pub lmstudio: Option<LmStudioSettingsContent>,
    pub mistral: Option<MistralSettingsContent>,
    pub ollama: Option<OllamaSettingsContent>,
    pub opencode: Option<OpenCodeSettingsContent>,
    pub open_router: Option<OpenRouterSettingsContent>,
    pub openai: Option<OpenAiSettingsContent>,
    pub openai_compatible: Option<HashMap<Arc<str>, OpenAiCompatibleSettingsContent>>,
    pub vercel_ai_gateway: Option<VercelAiGatewaySettingsContent>,
    pub x_ai: Option<XAiSettingsContent>,
    #[serde(rename = "zed.dev")]
    pub zed_dot_dev: Option<ZedDotDevSettingsContent>,
}

/// 完整的 LLM 配置（aa.json 旧格式）。
///
/// ```json
/// {
///   "model": "ollama/gemma4:31b-cloud",
///   "provider": { "ollama": { "base_url": "http://localhost:11434" } },
///   "mcp": { "servers": { ... } }
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// `provider/model` 格式的模型标识。
    #[serde(default)]
    pub model: Option<String>,
    /// 按 provider 名称索引的配置。
    #[serde(default)]
    pub provider: HashMap<String, ProviderConfig>,
    /// MCP 服务器配置。
    #[serde(default)]
    pub mcp: Option<McpConfig>,
}

/// 单个 provider 的配置。
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// MCP 服务器配置。
#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: HashMap<String, McpServerDef>,
}

/// 单个 MCP 服务器定义。
#[derive(Debug, Clone, Deserialize)]
pub struct McpServerDef {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// 解析后的运行配置（所有字段都已填充默认值）。
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// provider 标识，如 `"openai"` 或 `"ollama"`。
    pub provider: String,
    /// 模型名称，如 `"gpt-4o-mini"`。
    pub model: String,
    /// API key（可能为空）。
    pub api_key: String,
    /// API base URL。
    pub base_url: String,
}

impl Config {
    /// Get MCP servers as a JSON value (compatible with `McpToolProvider::from_json`),
    /// falling back to `AA_MCP_SERVERS` env var for backward compatibility.
    pub fn mcp_servers_json(&self) -> Option<serde_json::Value> {
        // aa.json has priority
        if let Some(mcp) = &self.mcp {
            if !mcp.servers.is_empty() {
                let map: serde_json::Map<String, serde_json::Value> = mcp
                    .servers
                    .iter()
                    .map(|(name, def)| {
                        let val = serde_json::json!({
                            "command": def.command,
                            "args": def.args,
                        });
                        (name.clone(), val)
                    })
                    .collect();
                return Some(serde_json::Value::Object(map));
            }
        }
        // Fall back to env var
        if let Ok(json_str) = std::env::var("AA_MCP_SERVERS") {
            if !json_str.is_empty() {
                if let Ok(val) = serde_json::from_str(&json_str) {
                    return Some(val);
                }
            }
        }
        None
    }

    /// 从标准位置加载 `aa.json`。
    ///
    /// 搜索路径（先找到就用）：
    /// 1. `./aa.json`
    /// 2. `$HOME/.config/aa/aa.json`
    pub fn load() -> Self {
        for path in config_paths() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str(&content) {
                    return cfg;
                }
            }
        }
        Config {
            model: None,
            provider: HashMap::default(),
            mcp: None,
        }
    }

    /// 将配置解析为运行时值。
    ///
    /// 优先级（高 → 低）：
    /// - CLI 参数（调用者传入的 `cli_*`）
    /// - `AA_MODEL` / `AA_PROVIDER` / `AA_API_KEY` / `AA_BASE_URL` env vars
    /// - 旧的 `AA_LLM_*` 环境变量（向后兼容）
    /// - `aa.json` 中的值
    /// - 硬编码默认值
    pub fn resolve(
        &self,
        cli_provider: Option<&str>,
        cli_model: Option<&str>,
        cli_base_url: Option<&str>,
    ) -> ResolvedConfig {
        // 决定 provider
        let provider = pick(
            cli_provider.map(String::from),
            env("AA_PROVIDER"),
            env("AA_LLM_PROVIDER"),
            self.model
                .as_ref()
                .and_then(|m| m.split_once('/'))
                .map(|(p, _)| p.to_string()),
            "ollama",
        );

        // 决定 model name
        let default_model = match provider.as_str() {
            "ollama" => "gemma4:31b-cloud",
            _ => "gpt-4o-mini",
        };

        let model = pick(
            cli_model.map(String::from),
            env("AA_MODEL"),
            env("AA_LLM_MODEL"),
            self.model.as_ref().map(|m| {
                m.split_once('/')
                    .map(|(_, m)| m.to_string())
                    .unwrap_or_else(|| m.clone())
            }),
            default_model,
        );

        // 决定 API key
        let api_key = pick(
            env("AA_API_KEY"),
            env("AA_LLM_API_KEY"),
            self.provider.get(&provider).and_then(|p| p.api_key.clone()),
            None,
            "",
        );

        // 决定 base URL
        let default_base_url = match provider.as_str() {
            "ollama" => "http://localhost:11434",
            _ => "https://api.openai.com/v1",
        };

        let base_url = pick(
            cli_base_url.map(String::from),
            env("AA_BASE_URL"),
            env("AA_LLM_BASE_URL"),
            self.provider
                .get(&provider)
                .and_then(|p| p.base_url.clone()),
            default_base_url,
        );

        ResolvedConfig {
            provider,
            model,
            api_key,
            base_url,
        }
    }
}

fn config_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from("aa.json")];
    if let Ok(home) = std::env::var("HOME") {
        paths.push(
            PathBuf::from(home)
                .join(".config")
                .join("aa")
                .join("aa.json"),
        );
    }
    paths
}

fn env(key: &str) -> Option<String> { std::env::var(key).ok().filter(|v| !v.is_empty()) }

/// 返回第一个 `Some` 值，全部为 `None` 时返回 `default`。
fn pick(
    a: Option<String>,
    b: Option<String>,
    c: Option<String>,
    d: Option<String>,
    default: &str,
) -> String {
    a.or(b).or(c).or(d).unwrap_or_else(|| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_defaults() {
        let cfg = Config {
            model: None,
            provider: HashMap::default(),
            mcp: None,
        };
        let resolved = cfg.resolve(None, None, None);
        assert_eq!(resolved.provider, "ollama");
        assert_eq!(resolved.model, "gemma4:31b-cloud");
        assert_eq!(resolved.base_url, "http://localhost:11434");
        assert_eq!(resolved.api_key, "");
    }

    #[test]
    fn test_resolve_from_model_string() {
        let cfg = Config {
            model: Some("ollama/llama3.2".into()),
            provider: HashMap::default(),
            mcp: None,
        };
        let resolved = cfg.resolve(None, None, None);
        assert_eq!(resolved.provider, "ollama");
        assert_eq!(resolved.model, "llama3.2");
        assert_eq!(resolved.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_resolve_model_without_provider_prefix() {
        let cfg = Config {
            model: Some("deepseek-chat".into()),
            provider: HashMap::default(),
            mcp: None,
        };
        let resolved = cfg.resolve(None, None, None);
        assert_eq!(resolved.provider, "ollama");
        assert_eq!(resolved.model, "deepseek-chat");
    }

    #[test]
    fn test_resolve_cli_overrides_file() {
        let cfg = Config {
            model: Some("ollama/llama3.2".into()),
            provider: HashMap::default(),
            mcp: None,
        };
        let resolved = cfg.resolve(
            Some("openai"),
            Some("gpt-4o"),
            Some("https://custom.com/v1"),
        );
        assert_eq!(resolved.provider, "openai");
        assert_eq!(resolved.model, "gpt-4o");
        assert_eq!(resolved.base_url, "https://custom.com/v1");
    }

    #[test]
    fn test_resolve_provider_config_from_file() {
        let mut provider = HashMap::default();
        provider.insert(
            "openai".into(),
            ProviderConfig {
                api_key: Some("sk-from-file".into()),
                base_url: Some("https://file-url.com/v1".into()),
            },
        );
        let cfg = Config {
            model: Some("openai/gpt-4o".into()),
            provider,
            mcp: None,
        };
        let resolved = cfg.resolve(None, None, None);
        assert_eq!(resolved.api_key, "sk-from-file");
        assert_eq!(resolved.base_url, "https://file-url.com/v1");
    }

    #[test]
    fn test_config_loading() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("aa.json");
        let mut f = std::fs::File::create(&config_path).unwrap();
        f.write_all(br#"{"model": "ollama/llama3.2", "provider": {"ollama": {"base_url": "http://localhost:12345"}}}"#).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let cfg = Config::load();
        std::env::set_current_dir(prev).unwrap();

        assert_eq!(cfg.model.as_deref(), Some("ollama/llama3.2"));
        assert_eq!(
            cfg.provider.get("ollama").unwrap().base_url.as_deref(),
            Some("http://localhost:12345")
        );
    }

    #[test]
    fn test_mcp_config_from_file() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("aa.json");
        let mut f = std::fs::File::create(&config_path).unwrap();
        f.write_all(br#"{"mcp": {"servers": {"my-tool": {"command": "npx", "args": ["-y", "mcp-server"]}}}}"#).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let cfg = Config::load();
        std::env::set_current_dir(prev).unwrap();

        let json = cfg.mcp_servers_json().unwrap();
        assert_eq!(json["my-tool"]["command"], "npx");
        assert_eq!(json["my-tool"]["args"][0], "-y");
        assert_eq!(json["my-tool"]["args"][1], "mcp-server");
    }

    #[test]
    fn test_mcp_config_fallback_env() {
        let cfg = Config {
            model: None,
            provider: HashMap::default(),
            mcp: None,
        };
        assert!(cfg.mcp_servers_json().is_none());

        // SAFETY: test-only code
        unsafe {
            std::env::set_var(
                "AA_MCP_SERVERS",
                r#"{"env-tool": {"command": "docker", "args": []}}"#,
            )
        };
        let json = cfg.mcp_servers_json().unwrap();
        assert_eq!(json["env-tool"]["command"], "docker");
        unsafe { std::env::remove_var("AA_MCP_SERVERS") };
    }
}
