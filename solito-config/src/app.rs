use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use super::renderer::{RendererConfig, WindowBackdrop};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};

const APP_DIR_NAME: &str = "Solito";

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub tracing: TracingConfig,
    pub shell: ShellConfig,
    pub window: WindowConfig,
    pub(crate) font: FontConfig,
}

impl AppConfig {
    pub fn load_or_create() -> Result<Self, Box<dyn Error>> {
        let paths: AppPaths = AppPaths::resolve()?;
        paths.ensure_dirs()?;

        if !paths.config_file.exists() {
            let default_config: String = toml::to_string_pretty(&Self::default())?;
            fs::write(&paths.config_file, default_config)?;
        }

        let config_text: String = fs::read_to_string(&paths.config_file)?;
        let config: Self = toml::from_str::<Self>(&config_text)?.sanitized();

        Ok(config)
    }

    pub fn renderer_config(&self) -> RendererConfig {
        RendererConfig {
            font_family: self.font.family.clone(),
            font_size: self.font.size,
            line_height: self.font.line_height,
            window_backdrop: self.window.backdrop.into(),
            window_acrylic_tint: self.window.acrylic_tint_tuple(),
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
            program: "nu".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
    pub(crate) backdrop: BackdropConfig,
    pub(crate) acrylic_tint: [u8; 4],
}

impl WindowConfig {
    fn sanitized(mut self) -> Self {
        let default: Self = Self::default();
        self.width = sanitize_positive_f32(self.width, default.width);
        self.height = sanitize_positive_f32(self.height, default.height);

        self
    }

    fn acrylic_tint_tuple(&self) -> (u8, u8, u8, u8) {
        (
            self.acrylic_tint[0],
            self.acrylic_tint[1],
            self.acrylic_tint[2],
            self.acrylic_tint[3],
        )
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        let (r, g, b, a): (u8, u8, u8, u8) = RendererConfig::DEFAULT_WINDOW_ACRYLIC_TINT;

        Self {
            width: 1000.0,
            height: 650.0,
            backdrop: BackdropConfig::Acrylic,
            acrylic_tint: [r, g, b, a],
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BackdropConfig {
    None,
    Transparent,
    #[default]
    Acrylic,
}

impl From<BackdropConfig> for WindowBackdrop {
    fn from(value: BackdropConfig) -> Self {
        match value {
            BackdropConfig::None => Self::None,
            BackdropConfig::Transparent => Self::Transparent,
            BackdropConfig::Acrylic => Self::Acrylic,
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
        let default: Self = Self::default();

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
    state_dir: PathBuf,
}

impl AppPaths {
    fn resolve() -> Result<Self, Box<dyn Error>> {
        let base_dirs: BaseDirs =
            BaseDirs::new().ok_or("failed to resolve user directories for config")?;
        let config_dir: PathBuf = base_dirs.data_local_dir().join(APP_DIR_NAME);
        let config_file: PathBuf = config_dir.join("config.toml");
        let state_dir: PathBuf = config_dir.clone();

        Ok(Self {
            config_dir,
            config_file,
            state_dir,
        })
    }

    fn ensure_dirs(&self) -> Result<(), Box<dyn Error>> {
        create_dir_all(&self.config_dir)?;
        create_dir_all(&self.state_dir)?;

        Ok(())
    }
}

fn create_dir_all(path: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(path)?;
    Ok(())
}

fn sanitize_positive_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, BackdropConfig};
    use crate::renderer::{RendererConfig, WindowBackdrop};

    #[test]
    fn partial_config_uses_defaults() {
        let config: AppConfig = toml::from_str(
            r#"
            [font]
            size = 16.0
            "#,
        )
        .unwrap();

        assert_eq!(config.shell.program, "nu");
        assert_eq!(config.font.family, RendererConfig::DEFAULT_FONT_FAMILY);
        assert_eq!(config.font.size, 16.0);
        assert_eq!(config.window.width, 1000.0);
    }

    #[test]
    fn renderer_config_maps_window_and_font_settings() {
        let config: AppConfig = toml::from_str(
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

        let renderer_config: RendererConfig = config.renderer_config();

        assert_eq!(renderer_config.font_family, "JetBrains Mono");
        assert_eq!(renderer_config.font_size, 18.0);
        assert_eq!(renderer_config.line_height, 28.0);
        assert_eq!(renderer_config.window_backdrop, WindowBackdrop::Transparent);
        assert_eq!(renderer_config.window_acrylic_tint, (1, 2, 3, 4));
    }

    #[test]
    fn invalid_config_values_fall_back_to_defaults() {
        let config: AppConfig = AppConfig {
            shell: super::ShellConfig {
                program: String::new(),
            },
            window: super::WindowConfig {
                width: 0.0,
                height: f32::NAN,
                backdrop: BackdropConfig::Acrylic,
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

        assert_eq!(config.shell.program, "nu");
        assert_eq!(config.window.width, 1000.0);
        assert_eq!(config.window.height, 650.0);
        assert_eq!(config.font.family, RendererConfig::DEFAULT_FONT_FAMILY);
        assert_eq!(config.font.size, RendererConfig::DEFAULT_FONT_SIZE);
        assert_eq!(config.font.line_height, RendererConfig::DEFAULT_LINE_HEIGHT);
        assert_eq!(config.tracing.filter, "error");
    }
}
