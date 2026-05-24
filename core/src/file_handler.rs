use anyhow::Context;
use anyhow::{Result, anyhow};
use mockall::automock;
use std::path::{Path, PathBuf};

#[automock]
/// File Handler trait
///
/// Provee una interfaz para interactuar con el sistema de archivos.
/// Normalmente utilizariamos algo como OsFileSystem para interactuar directamente, pero con
/// proposito de poder hacer testing de los metodos de esta libreria, se creo esta interfaz para
/// poder hacer una clase 'Mock' que no interactue con el FS.
pub trait FileHandler {
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn write(&self, path: &Path, content: &str) -> Result<()>;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    fn exists(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    fn file_name(&self, path: &Path) -> Option<String>;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
}

/// Implementador de FileHandler que si toca el sistema de archivos.
pub struct OsFileSystem;
impl FileHandler for OsFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path).with_context(|| format!("Error leyendo {:?}", path))
    }

    fn write(&self, path: &Path, content: &str) -> Result<()> {
        std::fs::write(path, content).with_context(|| format!("Error escribiendo {:?}", path))
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let entries = std::fs::read_dir(path)?;
        Ok(entries.filter_map(|e| e.ok().map(|d| d.path())).collect())
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn file_name(&self, path: &Path) -> Option<String> {
        path.file_name().map(|n| n.to_string_lossy().to_string())
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        path.canonicalize()
            .map_err(|e| anyhow!("Error canonicalizing path {:?}: {}", path, e))
    }
}
