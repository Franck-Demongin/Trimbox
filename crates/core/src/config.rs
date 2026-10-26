use directories::ProjectDirs;
use std::path::PathBuf;

const QUALIFIER: &str = "io.github";
const ORGANISATION: &str = "Franck-Demongin";
const APPLICATION: &str = "Trimbox";

/// Retourne l'espace `ProjectDirs` de l'application, si supporté par l'environnement.
pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORGANISATION, APPLICATION)
}

/// Répertoire contenant la configuration utilisateur.
pub fn config_dir() -> Option<PathBuf> {
    project_dirs().map(|dirs| dirs.config_dir().to_path_buf())
}

/// Fichier de configuration applicatif.
pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_dirs_are_consistent() {
        if let Some(path) = config_path() {
            assert!(path.ends_with("config.toml"));
        }
    }
}
