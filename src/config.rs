use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub encodings: EncodingConfig,
    #[serde(default)]
    pub matching: MatchingConfig,
    #[serde(default)]
    pub behaviour: BehaviourConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_encoding")]
    pub default_encoding: String,
    #[serde(default = "default_true")]
    pub confirm_overwrite: bool,
    #[serde(default = "default_clipboard_timeout")]
    pub clipboard_timeout_ms: u64,
    #[serde(default = "default_max_size")]
    pub max_file_size_mb: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EncodingConfig {
    #[serde(default = "default_bat_enc")]
    pub bat: String,
    #[serde(default = "default_ps1_enc")]
    pub ps1: String,
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchingConfig {
    #[serde(default = "default_true")]
    pub warn_on_mismatch: bool,
    #[serde(default)]
    pub auto_correct_ext: bool,
    #[serde(default = "default_true")]
    pub show_content_preview: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BehaviourConfig {
    #[serde(default = "default_ask")]
    pub default_on_conflict: String,
    #[serde(default = "default_ask")]
    pub default_on_mismatch: String,
}

// ---------- default value helpers ----------

fn default_encoding() -> String {
    "utf8".to_string()
}
fn default_true() -> bool {
    true
}
fn default_clipboard_timeout() -> u64 {
    2000
}
fn default_max_size() -> u64 {
    100
}
fn default_bat_enc() -> String {
    "gbk".to_string()
}
fn default_ps1_enc() -> String {
    "utf8bom".to_string()
}
fn default_ask() -> String {
    "ask".to_string()
}

// ---------- Default trait impls ----------



impl Default for CoreConfig {
    fn default() -> Self {
        CoreConfig {
            default_encoding: "utf8".to_string(),
            confirm_overwrite: true,
            clipboard_timeout_ms: 2000,
            max_file_size_mb: 100,
        }
    }
}

impl Default for EncodingConfig {
    fn default() -> Self {
        EncodingConfig {
            bat: "gbk".to_string(),
            ps1: "utf8bom".to_string(),
            custom: HashMap::new(),
        }
    }
}

impl Default for MatchingConfig {
    fn default() -> Self {
        MatchingConfig {
            warn_on_mismatch: true,
            auto_correct_ext: false,
            show_content_preview: true,
        }
    }
}

impl Default for BehaviourConfig {
    fn default() -> Self {
        BehaviourConfig {
            default_on_conflict: "ask".to_string(),
            default_on_mismatch: "ask".to_string(),
        }
    }
}

// ---------- loading logic ----------

impl Config {
    /// Load config with 3-tier priority: project > user > default
    pub fn load() -> Config {
        let mut config = Config::default();

        if let Ok(user_config) = Self::load_from_path(&Self::user_config_path()) {
            config = merge_configs(config, user_config);
        }

        if let Ok(project_config) = Self::load_from_path(&Self::project_config_path()) {
            config = merge_configs(config, project_config);
        }

        config
    }

    fn user_config_path() -> PathBuf {
        #[cfg(windows)]
        {
            let base =
                std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());
            PathBuf::from(base).join(".mfconfig")
        }
        #[cfg(not(windows))]
        {
            let base = std::env::var("HOME").unwrap_or_else(|_| "/home/default".to_string());
            PathBuf::from(base).join(".mfconfig")
        }
    }

    fn project_config_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_default()
            .join(".mf")
            .join("config.toml")
    }

    fn load_from_path(path: &std::path::Path) -> Result<Config, String> {
        if !path.exists() {
            return Err("File not found".to_string());
        }
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("无法读取配置文件: {}", e))?;
        let config: Config =
            toml::from_str(&content).map_err(|e| format!("配置文件解析失败: {}", e))?;
        Ok(config)
    }
}

// ---------- merge ----------

fn merge_configs(mut base: Config, overlay: Config) -> Config {
    base.core = overlay.core;
    base.matching = overlay.matching;
    base.behaviour = overlay.behaviour;
    base.encodings.bat = overlay.encodings.bat;
    base.encodings.ps1 = overlay.encodings.ps1;
    base.encodings.custom.extend(overlay.encodings.custom);
    base
}

// ---------- tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.core.default_encoding, "utf8");
        assert!(config.core.confirm_overwrite);
        assert_eq!(config.core.clipboard_timeout_ms, 2000);
        assert_eq!(config.core.max_file_size_mb, 100);
        assert_eq!(config.encodings.bat, "gbk");
        assert_eq!(config.encodings.ps1, "utf8bom");
        assert!(config.matching.warn_on_mismatch);
        assert!(!config.matching.auto_correct_ext);
        assert_eq!(config.behaviour.default_on_conflict, "ask");
    }

    #[test]
    fn test_config_roundtrip() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join(".mf");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let toml_content = r#"
[core]
default_encoding = "gbk"
max_file_size_mb = 200

[encodings]
bat = "utf8"
custom = { myext = "utf16le" }

[matching]
auto_correct_ext = true

[behaviour]
default_on_conflict = "overwrite"
"#;
        std::fs::write(&config_path, toml_content).unwrap();

        let config = Config::load_from_path(&config_path).unwrap();
        assert_eq!(config.core.default_encoding, "gbk");
        assert_eq!(config.core.max_file_size_mb, 200);
        assert_eq!(config.encodings.bat, "utf8");
        assert!(config.matching.auto_correct_ext);
        assert_eq!(config.behaviour.default_on_conflict, "overwrite");
        // Should have defaults for unspecified fields
        assert!(config.core.confirm_overwrite);
        assert_eq!(config.core.clipboard_timeout_ms, 2000);
    }

    #[test]
    fn test_load_nonexistent_path() {
        let result = Config::load_from_path(Path::new("/nonexistent/path.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "invalid toml {{{").unwrap();
        let result = Config::load_from_path(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_configs_overlay_overrides_base() {
        let base = Config::default();
        let toml_str = r#"
[core]
default_encoding = "utf16le"
confirm_overwrite = false

[encodings]
bat = "utf8"
custom = { a = "1", b = "2" }
"#;
        let overlay: Config = toml::from_str(toml_str).unwrap();
        let merged = merge_configs(base, overlay);

        assert_eq!(merged.core.default_encoding, "utf16le");
        assert!(!merged.core.confirm_overwrite);
        // Should keep defaults for unspecified overlay fields
        assert_eq!(merged.core.clipboard_timeout_ms, 2000);
        assert_eq!(merged.core.max_file_size_mb, 100);
        assert_eq!(merged.encodings.bat, "utf8");
        assert_eq!(merged.encodings.custom.len(), 2);
    }

    #[test]
    fn test_merge_custom_encoding_maps() {
        let base_str = r#"
[encodings.custom]
a = "1"
b = "2"
"#;
        let overlay_str = r#"
[encodings.custom]
b = "overridden"
c = "3"
"#;
        let base: Config = toml::from_str(base_str).unwrap();
        let overlay: Config = toml::from_str(overlay_str).unwrap();
        let merged = merge_configs(base, overlay);

        assert_eq!(merged.encodings.custom.get("a").unwrap(), "1");
        assert_eq!(merged.encodings.custom.get("b").unwrap(), "overridden");
        assert_eq!(merged.encodings.custom.get("c").unwrap(), "3");
        assert_eq!(merged.encodings.custom.len(), 3);
    }

    #[test]
    fn test_load_graceful_no_config_files() {
        // Should return defaults when no config files exist
        let config = Config::load();
        assert_eq!(config.core.default_encoding, "utf8");
    }
}
