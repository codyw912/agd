use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AgdPaths {
    pub home: PathBuf,
}

impl AgdPaths {
    pub fn from_env() -> Result<Self> {
        let home = match env::var_os("AGD_HOME") {
            Some(value) => PathBuf::from(value),
            None => dirs::home_dir()
                .context("could not determine home directory")?
                .join(".agd"),
        };
        Ok(Self { home })
    }

    pub fn project_dir(&self, project_id: &str) -> PathBuf {
        self.home.join("projects").join(project_id)
    }

    pub fn project_file(&self, project_id: &str) -> PathBuf {
        self.project_dir(project_id).join("project.json")
    }
}
