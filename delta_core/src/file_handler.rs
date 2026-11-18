use anyhow::Context;
use anyhow::{Result, anyhow};
use mockall::automock;
use std::cell::RefCell;
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

/// Envoltorio de un `FileHandler` que registra cada escritura.
///
/// Las lecturas se delegan al handler interno. Las escrituras se anotan en `written()` y solo
/// llegan al handler interno si `dry_run` es `false`. Sirve para hacer dry-run de un bump y para
/// reportar que archivos fueron modificados.
pub struct RecordingFileHandler<'a, F: FileHandler> {
    inner: &'a F,
    dry_run: bool,
    writes: RefCell<Vec<(PathBuf, String)>>,
}

impl<'a, F: FileHandler> RecordingFileHandler<'a, F> {
    pub fn new(inner: &'a F, dry_run: bool) -> Self {
        Self {
            inner,
            dry_run,
            writes: RefCell::new(Vec::new()),
        }
    }

    /// Rutas escritas (o que se hubieran escrito en dry-run), en orden.
    pub fn written(&self) -> Vec<PathBuf> {
        self.writes
            .borrow()
            .iter()
            .map(|(p, _)| p.clone())
            .collect()
    }

    /// Rutas y contenido escrito (o que se hubiera escrito en dry-run), en orden.
    pub fn writes(&self) -> Vec<(PathBuf, String)> {
        self.writes.borrow().clone()
    }
}

impl<F: FileHandler> FileHandler for RecordingFileHandler<'_, F> {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        self.inner.read_to_string(path)
    }

    fn write(&self, path: &Path, content: &str) -> Result<()> {
        if !self.dry_run {
            self.inner.write(path, content)?;
        }
        self.writes
            .borrow_mut()
            .push((path.to_path_buf(), content.to_string()));
        Ok(())
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        self.inner.read_dir(path)
    }

    fn exists(&self, path: &Path) -> bool {
        self.inner.exists(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.inner.is_dir(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.inner.is_file(path)
    }

    fn file_name(&self, path: &Path) -> Option<String> {
        self.inner.file_name(path)
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        self.inner.canonicalize(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_records_without_writing() {
        let mock = MockFileHandler::new(); // sin expect_write: escribir haria panic
        let rec = RecordingFileHandler::new(&mock, true);
        rec.write(Path::new("a.txt"), "hola").unwrap();
        assert_eq!(rec.written(), vec![PathBuf::from("a.txt")]);
        assert_eq!(rec.writes()[0].1, "hola");
    }

    #[test]
    fn real_run_writes_and_records() {
        let mut mock = MockFileHandler::new();
        mock.expect_write().times(1).returning(|_, _| Ok(()));
        let rec = RecordingFileHandler::new(&mock, false);
        rec.write(Path::new("b.txt"), "x").unwrap();
        assert_eq!(rec.written(), vec![PathBuf::from("b.txt")]);
    }

    #[test]
    fn real_run_propagates_write_error_and_does_not_record() {
        let mut mock = MockFileHandler::new();
        mock.expect_write().returning(|_, _| Err(anyhow!("denied")));
        let rec = RecordingFileHandler::new(&mock, false);
        assert!(rec.write(Path::new("c.txt"), "x").is_err());
        assert!(rec.written().is_empty());
    }
}
