use std::{env, fs, path::PathBuf};

use super::renderer::{RendererConfig, WindowBackdrop, sanitize_positive_f32};
use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};

const APP_DIR_NAME: &str = "Solito";
const SHELL_PROGRAM_ENV: &str = "SOLITO_SHELL_PROGRAM";

#[cfg(target_os = "windows")]
const DEFAULT_SHELL_PROGRAM: &str = "pwsh";
#[cfg(target_os = "linux")]
const DEFAULT_SHELL_PROGRAM: &str = "bash";
#[cfg(target_os = "macos")]
const DEFAULT_SHELL_PROGRAM: &str = "zsh";
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
const DEFAULT_SHELL_PROGRAM: &str = "sh";

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub tracing: TracingConfig,
    pub shell: ShellConfig,
    pub window: WindowConfig,
    pub(crate) font: FontConfig,
}

impl AppConfig {
    pub fn load_or_create() -> Result<Self> {
        let paths = AppPaths::resolve()?;
        paths.ensure_config_dir()?;

        if !paths.config_file.exists() {
            let default_config = toml::to_string_pretty(&Self::default())?;
            fs::write(&paths.config_file, default_config)?;
        }

        let config_text = fs::read_to_string(&paths.config_file)?;
        let shell_program =
            env::var_os(SHELL_PROGRAM_ENV).map(|program| program.to_string_lossy().into_owned());
        let config = toml::from_str::<Self>(&config_text)?
            .sanitized()
            .with_shell_program(shell_program.as_deref());

        Ok(config)
    }

    pub fn renderer_config(&self) -> RendererConfig {
        let [r, g, b, a] = self.window.acrylic_tint;

        RendererConfig {
            font_family: self.font.family.clone(),
            font_size: self.font.size,
            line_height: self.font.line_height,
            window_backdrop: self.window.backdrop,
            window_acrylic_tint: (r, g, b, a),
        }
        .sanitized()
    }

    fn sanitized(mut self) -> Self {
        self.shell = self.shell.sanitized();
        self.window = self.window.sanitized();
        self.font = self.font.sanitized();
        self.tracing = self.tracing.sanitized();

        self
    }

    fn with_shell_program(mut self, program: Option<&str>) -> Self {
        // Runtime overrides let tools launch a one-off shell without rewriting
        // the user's persistent config file.
        if let Some(program) = program.filter(|program| !program.trim().is_empty()) {
            self.shell.program = program.to_string();
        }

        self
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ShellConfig {
    pub program: String,
}

impl ShellConfig {
    fn sanitized(mut self) -> Self {
        if self.program.trim().is_empty() {
            self.program = Self::default().program;
        }

        self
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            program: DEFAULT_SHELL_PROGRAM.to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
    pub(crate) backdrop: WindowBackdrop,
    pub(crate) acrylic_tint: [u8; 4],
}

impl WindowConfig {
    fn sanitized(mut self) -> Self {
        let default = Self::default();
        self.width = sanitize_positive_f32(self.width, default.width);
        self.height = sanitize_positive_f32(self.height, default.height);

        self
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        let (r, g, b, a): (u8, u8, u8, u8) = RendererConfig::DEFAULT_WINDOW_ACRYLIC_TINT;

        Self {
            width: 1000.0,
            height: 650.0,
            backdrop: WindowBackdrop::Acrylic,
            acrylic_tint: [r, g, b, a],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct FontConfig {
    pub(crate) family: String,
    pub(crate) size: f32,
    pub(crate) line_height: f32,
}

impl FontConfig {
    fn sanitized(mut self) -> Self {
        let default = Self::default();

        if self.family.trim().is_empty() {
            self.family = default.family;
        }

        self.size = sanitize_positive_f32(self.size, default.size);
        self.line_height = sanitize_positive_f32(self.line_height, default.line_height);

        self
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: RendererConfig::DEFAULT_FONT_FAMILY.to_string(),
            size: RendererConfig::DEFAULT_FONT_SIZE,
            line_height: RendererConfig::DEFAULT_LINE_HEIGHT,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TracingConfig {
    pub filter: String,
}

impl TracingConfig {
    fn sanitized(mut self) -> Self {
        if self.filter.trim().is_empty() {
            self.filter = Self::default().filter;
        }

        self
    }
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            filter: "error".to_string(),
        }
    }
}

#[derive(Debug)]
struct AppPaths {
    config_dir: PathBuf,
    config_file: PathBuf,
}

impl AppPaths {
    fn resolve() -> Result<Self> {
        let base_dirs = BaseDirs::new().context("failed to resolve user directories for config")?;
        let config_dir = base_dirs.data_local_dir().join(APP_DIR_NAME);
        let config_file = config_dir.join("config.toml");

        Ok(Self {
            config_dir,
            config_file,
        })
    }

    fn ensure_config_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.config_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, ShellConfig};
    use crate::renderer::{RendererConfig, WindowBackdrop};

    #[test]
    fn partial_config_uses_defaults() {
        let config = toml::from_str::<AppConfig>(
            r#"
            [font]
            size = 16.0
            "#,
        )
        .unwrap();

        assert_eq!(config.shell.program, ShellConfig::default().program);
        assert_eq!(config.font.family, RendererConfig::DEFAULT_FONT_FAMILY);
        assert_eq!(config.font.size, 16.0);
        assert_eq!(config.window.width, 1000.0);
    }

    #[test]
    fn renderer_config_maps_window_and_font_settings() {
        let config = toml::from_str::<AppConfig>(
            r#"
            [window]
            backdrop = "transparent"
            acrylic_tint = [1, 2, 3, 4]

            [font]
            family = "JetBrains Mono"
            size = 18.0
            line_height = 28.0
            "#,
        )
        .unwrap();

        let renderer_config = config.renderer_config();

        assert_eq!(renderer_config.font_family, "JetBrains Mono");
        assert_eq!(renderer_config.font_size, 18.0);
        assert_eq!(renderer_config.line_height, 28.0);
        assert_eq!(renderer_config.window_backdrop, WindowBackdrop::Transparent);
        assert_eq!(renderer_config.window_acrylic_tint, (1, 2, 3, 4));
    }

    #[test]
    fn backdrop_variants_keep_their_toml_names() {
        for (name, expected) in [
            ("none", WindowBackdrop::None),
            ("transparent", WindowBackdrop::Transparent),
            ("acrylic", WindowBackdrop::Acrylic),
        ] {
            let config =
                toml::from_str::<AppConfig>(&format!("[window]\nbackdrop = \"{name}\"")).unwrap();

            assert_eq!(config.window.backdrop, expected);
        }
    }

    #[test]
    fn default_config_round_trips_through_toml() {
        let encoded = toml::to_string_pretty(&AppConfig::default()).unwrap();
        let config = toml::from_str::<AppConfig>(&encoded).unwrap();

        assert!(encoded.contains("backdrop = \"acrylic\""));
        assert!(encoded.contains("acrylic_tint = ["));
        assert_eq!(config.window.backdrop, WindowBackdrop::Acrylic);
        assert_eq!(config.window.acrylic_tint, [18, 18, 18, 190]);
    }

    #[test]
    fn invalid_config_values_fall_back_to_defaults() {
        let config = AppConfig {
            shell: ShellConfig {
                program: String::new(),
            },
            window: super::WindowConfig {
                width: 0.0,
                height: f32::NAN,
                backdrop: WindowBackdrop::Acrylic,
                acrylic_tint: [18, 18, 18, 190],
            },
            font: super::FontConfig {
                family: " ".to_string(),
                size: -1.0,
                line_height: 0.0,
            },
            tracing: super::TracingConfig {
                filter: String::new(),
            },
        }
        .sanitized();

        assert_eq!(config.shell.program, ShellConfig::default().program);
        assert_eq!(config.window.width, 1000.0);
        assert_eq!(config.window.height, 650.0);
        assert_eq!(config.font.family, RendererConfig::DEFAULT_FONT_FAMILY);
        assert_eq!(config.font.size, RendererConfig::DEFAULT_FONT_SIZE);
        assert_eq!(config.font.line_height, RendererConfig::DEFAULT_LINE_HEIGHT);
        assert_eq!(config.tracing.filter, "error");
    }

    #[test]
    fn shell_default_matches_the_target_platform() {
        let program = ShellConfig::default().program;

        #[cfg(target_os = "windows")]
        assert_eq!(program, "pwsh");
        #[cfg(target_os = "linux")]
        assert_eq!(program, "bash");
        #[cfg(target_os = "macos")]
        assert_eq!(program, "zsh");
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        assert_eq!(program, "sh");
    }

    #[test]
    fn explicit_shell_program_is_preserved() {
        let config = toml::from_str::<AppConfig>(
            r#"
            [shell]
            program = "nu"
            "#,
        )
        .unwrap()
        .sanitized();

        assert_eq!(config.shell.program, "nu");
    }

    #[test]
    fn runtime_shell_override_does_not_change_other_config() {
        let config = toml::from_str::<AppConfig>(
            r#"
            [shell]
            program = "nu"

            [window]
            width = 900.0
            "#,
        )
        .unwrap()
        .sanitized()
        .with_shell_program(Some("C:/tools/solito-bench.exe"));

        assert_eq!(config.shell.program, "C:/tools/solito-bench.exe");
        assert_eq!(config.window.width, 900.0);
    }

    #[test]
    fn blank_runtime_shell_override_is_ignored() {
        let config = AppConfig::default().with_shell_program(Some("  "));

        assert_eq!(config.shell.program, ShellConfig::default().program);
    }
}
