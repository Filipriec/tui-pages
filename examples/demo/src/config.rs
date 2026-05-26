use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub keybindings: Keybindings,
}

#[derive(Debug, Deserialize)]
pub struct Keybindings {
    #[serde(default)]
    pub quit: Vec<String>,
    #[serde(default)]
    pub focus_next: Vec<String>,
    #[serde(default)]
    pub focus_prev: Vec<String>,
    #[serde(default)]
    pub move_up: Vec<String>,
    #[serde(default)]
    pub move_down: Vec<String>,
    #[serde(default)]
    pub select: Vec<String>,
    #[serde(default)]
    pub home: Vec<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub details: Vec<String>,
    #[serde(default)]
    pub previous_buffer: Vec<String>,
    #[serde(default)]
    pub next_buffer: Vec<String>,
    #[serde(default)]
    pub split_pane: Vec<String>,
    #[serde(default)]
    pub next_pane: Vec<String>,
    #[serde(default)]
    pub previous_pane: Vec<String>,
    #[serde(default)]
    pub close_pane: Vec<String>,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            quit: vec!["ctrl+q".into(), "ctrl+c".into(), "esc".into()],
            focus_next: vec!["tab".into()],
            focus_prev: vec!["shift+tab".into(), "ctrl+p".into()],
            move_up: vec!["up".into(), "k".into()],
            move_down: vec!["down".into(), "j".into()],
            select: vec!["enter".into()],
            home: vec!["1".into()],
            options: vec!["2".into()],
            details: vec!["3".into()],
            previous_buffer: vec!["[".into()],
            next_buffer: vec!["]".into()],
            split_pane: vec!["v".into()],
            next_pane: vec!["p".into()],
            previous_pane: vec!["shift+p".into()],
            close_pane: vec!["x".into()],
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.toml");
        let fallback = Self {
            keybindings: Keybindings::default(),
        };

        let Ok(content) = std::fs::read_to_string(&config_path) else {
            return fallback;
        };

        toml::from_str(&content).unwrap_or(fallback)
    }
}
