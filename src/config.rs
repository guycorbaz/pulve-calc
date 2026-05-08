use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub tracteur: TracteurConfig,
    pub pulverisateur: PulverisateurConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracteurConfig {
    pub nom: String,
    pub moteur: String,
    pub regime_max: f64,
    pub vitesses_max: Vec<f64>,
    pub pto: PtoConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtoConfig {
    pub regime_nominal: f64,
    pub regime_moteur_nominal: f64,
    pub pto_min: f64,
    pub pto_max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulverisateurConfig {
    pub nom: String,
    pub nombre_buses: u32,
    pub type_buses: String,
    pub largeur_travail: f64,
    pub citerne: f64,
    pub etalonnage: Etalonnage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Etalonnage {
    pub pressions: Vec<f64>,
    pub debits_par_buse: Vec<f64>,
}

const DEFAULT_CONFIG: &str = include_str!("../config/default.toml");

impl Config {
    /// Charge la config. Retourne (config, avertissement éventuel)
    pub fn load() -> (Self, Option<String>) {
        let config_path = Self::config_path();
        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(config) => return (config, None),
                    Err(e) => {
                        let msg = format!(
                            "Config corrompue ({}), utilisation des défauts: {}",
                            config_path.display(),
                            e
                        );
                        eprintln!("{msg}");
                        return (Self::default_config(), Some(msg));
                    }
                },
                Err(e) => {
                    let msg = format!("Impossible de lire {}: {}", config_path.display(), e);
                    eprintln!("{msg}");
                    return (Self::default_config(), Some(msg));
                }
            }
        }
        (Self::default_config(), None)
    }

    pub fn save(&self) -> Result<(), String> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&config_path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("pulve-calc").join("config.toml")
        } else {
            PathBuf::from("config/default.toml")
        }
    }

    fn default_config() -> Self {
        toml::from_str(DEFAULT_CONFIG).expect("Default config should be valid")
    }
}
