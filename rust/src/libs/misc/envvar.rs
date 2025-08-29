use std::collections::HashMap;
use std::env;
use std::fs;

/// Environment variable utility
pub struct Envvar;

impl Envvar {
    /// Get environment variable by name, or default if not set.
    pub fn get(varname: &str, default: Option<&str>) -> Option<String> {
        match env::var(varname) {
            Ok(val) => Some(val),
            Err(_) => default.map(|d| d.to_string()),
        }
    }

    /// Check if environment variable is set.
    pub fn is_set(varname: &str) -> bool {
        env::var(varname).is_ok()
    }

    /// Check if all environment variables in list are set.
    pub fn is_set_all(vars: &[&str]) -> bool {
        vars.iter().all(|&v| Self::is_set(v))
    }

    /// Check if any environment variable in list is set.
    pub fn is_set_any(vars: &[&str]) -> bool {
        vars.iter().any(|&v| Self::is_set(v))
    }

    /// Set environment variable.
    pub fn set(varname: &str, value: &str) {
        env::set_var(varname, value);
    }

    /// Load `.env`-style file into environment.
    pub fn load(filename: &str) -> HashMap<String, String> {
        let envmap = Self::read(filename);
        for (k, v) in &envmap {
            env::set_var(k, v);
        }
        envmap
    }

    /// Require `.env` file or return error.
    pub fn require_env_file(filename: &str) -> Result<HashMap<String, String>, ()> {
        if std::path::Path::new(filename).exists() {
            Ok(Self::load(filename))
        } else {
            Err(())
        }
    }

    /// Get all keys in `.env` file.
    pub fn keys(filename: &str) -> Vec<String> {
        Self::read(filename).keys().cloned().collect()
    }

    /// Get all values in `.env` file.
    pub fn values(filename: &str) -> Vec<String> {
        Self::read(filename).values().cloned().collect()
    }

    /// Read `.env` file into HashMap
    pub fn read(path: &str) -> HashMap<String, String> {
        let data = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Could not read env file: {}", path));

        data.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().replace("export ", "").replace('\'', ""))
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 && !parts[1].is_empty() {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect()
    }
}
