pub use crate::file_handler::{FileHandler, OsFileSystem, RecordingFileHandler};
use anyhow::Context;
use anyhow::{Result, anyhow};
use chrono::Local;
use regex::Regex;
use std::fmt;
use std::path::{Path, PathBuf};

pub mod file_handler;
pub mod navigation;

pub use navigation::{DirectoryEntry, FileType, FuzzyMatch, NavigationError, NavigationService};

#[derive(Debug, Clone, Copy, PartialEq)]
/// Formato de versionamiento de los proyectos de Fin700.
///
/// Para nosotros, `major` *siempre* deberia ser 8. `Minor` y `Patch` son valores entre 0 y 99 inclusive,
/// y `build` es siempre cero exceptuando cuando es generado por un pipeline.
pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: u32,
}

impl AppVersion {
    /// Sube en 1 la version de un proyecto.
    ///
    /// Por regla de negocio, la forma en la que se sube de version es subiendo el `patch` hasta 99.
    /// Si este pasa a 100, en vez de eso, subimos de version el `minor` y devolvemos `patch` hacia 0.
    pub fn bumped(&self) -> Self {
        let mut next = *self;
        next.patch += 1;
        if next.patch >= 100 {
            next.minor += 1;
            next.patch = 0;
        }
        next
    }

    /// Parsea una version.
    ///
    /// Se espera en el formato de: "{major}.{minor}.{patch}.{build}". Lo esperable es que sean u16,
    /// pero lo deje en u32 por flojera nomas. En cualquier caso, si el numero no es parseable como
    /// u32, entonces no es una version valida.
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return Err(anyhow!("Version must follow X.Y.Z.W format (found: {})", s));
        }
        Ok(Self {
            major: parts[0].parse()?,
            minor: parts[1].parse()?,
            patch: parts[2].parse()?,
            build: parts[3].parse()?,
        })
    }
}

impl fmt::Display for AppVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.major, self.minor, self.patch, self.build
        )
    }
}

#[derive(Default)]
pub struct ProjectVersions {
    pub web: Option<AppVersion>,
    pub bl: Option<AppVersion>,
}

/// Intenta leer las versiones del proeycto seleccionado.
pub fn get_current_versions(
    fs: &impl FileHandler,
    project_dir: &Path,
    project_name: &str,
) -> ProjectVersions {
    let web_path = project_dir
        .join(format!("web{}", project_name))
        .join("package.json");

    let bl_path = project_dir
        .join(format!("bl{}", project_name))
        .join(format!("bl{}", project_name))
        .join(format!("bl{}.csproj", project_name));

    ProjectVersions {
        web: extract_version_from_package_json(fs, &web_path),
        bl: extract_version_from_csproj(fs, &bl_path),
    }
}

/// A partir de un regex, obtiene la version del `package.json`.
fn extract_version_from_package_json(fs: &impl FileHandler, path: &Path) -> Option<AppVersion> {
    let content = fs.read_to_string(path).ok()?;
    let re = Regex::new(r#""version"\s*:\s*"([^"]+)""#).ok()?;
    let caps = re.captures(&content)?;
    AppVersion::parse(caps.get(1)?.as_str()).ok()
}

/// A partir de un Regex, obtiene la version del archivo `.csproj`
fn extract_version_from_csproj(fs: &impl FileHandler, path: &Path) -> Option<AppVersion> {
    let content = fs.read_to_string(path).ok()?;
    let re = Regex::new(r"(?i)<Version>(.*?)</Version>").ok()?;
    let caps = re.captures(&content)?;
    AppVersion::parse(caps.get(1)?.as_str()).ok()
}

#[derive(Debug)]
/// Estructura que guarda las intrucciones para escribir un Bump dentro de README.md
pub struct BumpInstructions {
    /// Autor del bump
    pub author: String,

    /// Ruta del proyecto que se esta escribiendo
    pub project_dir: PathBuf,

    /// Nombre del proyecto a escribir
    pub project_name: String,

    /// Objeto que fue modificado.
    pub object: String,

    /// Descripcion o mensaje del bump.
    pub description: String,

    /// Version original del proyecto
    pub version_original: AppVersion,

    /// Indicador del tipo de proyecto que vamos a bumpear
    pub is_web_project: bool,
}

impl BumpInstructions {
    /// Ejecuta el bumpeo de un proyecto.
    ///
    /// Para bumpear un proyecto, se siguen los siguientes pasos:
    /// - Se otiene la version objetivo y la Fecha.
    /// - Se determina si es web o bl.
    /// - Se actualiza el Readme a partir del BumpInstructions
    /// - Dependiendo del tipo de proyecto, se actualiza package.json o .csproj
    ///
    /// Primero calcula todo en memoria (`plan_bump`), luego escribe los archivos de version y el
    /// README al final. Si una escritura falla, restaura los archivos ya escritos, para no dejar
    /// un README con una version que no se aplico (y que un reintento duplicaria).
    ///
    /// Devuelve las rutas escritas, en orden.
    pub fn execute_bump(&self, fs: &impl FileHandler) -> Result<Vec<PathBuf>> {
        let writes = self.plan_bump(fs)?;

        // Contenido original para poder deshacer. El README va al final, nunca hay que deshacerlo.
        let originals: Vec<Option<String>> = writes
            .iter()
            .map(|(p, _)| fs.read_to_string(p).ok())
            .collect();

        for (i, (path, content)) in writes.iter().enumerate() {
            if let Err(err) = fs.write(path, content) {
                let mut rollback_errors = Vec::new();
                for (j, (done, _)) in writes[..i].iter().enumerate() {
                    if let Some(orig) = &originals[j]
                        && let Err(e) = fs.write(done, orig)
                    {
                        rollback_errors.push(format!("{:?}: {}", done, e));
                    }
                }
                if rollback_errors.is_empty() {
                    return Err(err.context("Bump cancelado, no se modifico ningun archivo"));
                }
                return Err(err.context(format!(
                    "Bump fallido y no se pudieron restaurar: {}",
                    rollback_errors.join("; ")
                )));
            }
        }

        Ok(writes.into_iter().map(|(p, _)| p).collect())
    }

    /// Calcula las escrituras de un bump sin tocar el disco: `(ruta, contenido nuevo)`, con los
    /// archivos de version primero y el README al final.
    pub fn plan_bump(&self, fs: &impl FileHandler) -> Result<Vec<(PathBuf, String)>> {
        let rec = RecordingFileHandler::new(fs, true);
        self.write_bump(&rec)?;
        let mut writes = rec.writes();
        // write_bump escribe el README primero; lo movemos al final.
        let readme_at = writes
            .iter()
            .position(|(p, _)| p.file_name().is_some_and(|n| n == "README.md"));
        if let Some(i) = readme_at {
            let readme = writes.remove(i);
            writes.push(readme);
        }
        Ok(writes)
    }

    fn write_bump(&self, fs: &impl FileHandler) -> Result<()> {
        let target_version = self.version_original.bumped();
        let date_str = Local::now().format("%d/%m/%Y").to_string();

        // Determine the "Objeto" name (e.g., "webProjectName" or "blProjectName")
        let prefix = if self.is_web_project { "web" } else { "bl" };
        let object_name = format!("{}{}", prefix, self.project_name);
        let project_path = self.project_dir.join(&self.project_name).join(&object_name);

        Self::update_readme(
            fs,
            &project_path.join("README.md"),
            &target_version.to_string(),
            &self.author,
            &self.object,
            &date_str,
            &self.description,
        )?;

        if self.is_web_project {
            Self::update_package_json(
                fs,
                &project_path.join("package.json"),
                &target_version.to_string(),
            )?;
        } else {
            let csproj_files = Self::find_csproj_shallow(fs, &project_path);
            for path in csproj_files {
                Self::update_csproj(fs, &path, &target_version.to_string())?;
            }
        }

        Ok(())
    }

    fn update_readme(
        fs: &impl FileHandler,
        path: &Path,
        ver: &str,
        aut: &str,
        obj: &str,
        date: &str,
        desc: &str,
    ) -> Result<()> {
        let entry = format!(
            "## Version {}\n\n\
        - Nombre: {}\n\
        - Objeto: {}\n\
        - Fecha: {}\n\
        - Descripción: {}\n",
            ver, aut, obj, date, desc
        );

        let existing = fs.read_to_string(path).unwrap_or_default();
        fs.write(path, &format!("{}{}", entry, existing))
            .with_context(|| format!("Failed to update README at {:?}", path))
    }

    fn update_package_json(fs: &impl FileHandler, path: &Path, version: &str) -> Result<()> {
        let content = fs.read_to_string(path)?.replace("\r\n", "\n");

        let re = Regex::new(r#"("version"\s*:\s*)"[^"]+""#).unwrap();
        let replacement = format!(r#"${{1}}"{version}""#);
        let updated = re.replace(&content, replacement.as_str());
        let updated = updated.as_ref();
        fs.write(path, updated)?;
        Ok(())
    }

    fn update_csproj(fs: &impl FileHandler, path: &Path, version: &str) -> Result<()> {
        let content = fs.read_to_string(path)?;
        let re = Regex::new(r"(?i)<Version>.*?</Version>").unwrap();
        let updated = re.replace(&content, format!("<Version>{}</Version>", version));
        fs.write(path, &updated)?;
        Ok(())
    }

    fn find_csproj_shallow(fs: &impl FileHandler, base_path: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let Ok(entries) = fs.read_dir(base_path) else {
            return files;
        };

        for path in entries {
            if path.is_dir() {
                if let Ok(sub_entries) = fs.read_dir(&path) {
                    for sub_path in sub_entries {
                        if Self::is_csproj(&sub_path) {
                            files.push(sub_path);
                        }
                    }
                }
            } else if Self::is_csproj(&path) {
                files.push(path);
            }
        }
        files
    }

    fn is_csproj(path: &Path) -> bool {
        path.extension().and_then(|s| s.to_str()) == Some("csproj")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use std::path::PathBuf;

    fn get_path(p: &str) -> PathBuf {
        p.split('/').collect()
    }

    #[test]
    fn test_web_bump_success() {
        let mut mock = crate::file_handler::MockFileHandler::new();
        // Path logic: project_dir / project_name / object_name
        let base_path = get_path("root/MyProj/webMyProj");
        let pkg_path = base_path.join("package.json");
        let readme_path = base_path.join("README.md");

        // 1.0.0.0 bumped is 1.0.1.0 (patch is the 3rd index in your AppVersion)
        let expected_ver = "1.0.1.0";

        mock.expect_read_to_string()
            .with(eq(pkg_path.clone()))
            .returning(|_| Ok(r#" "version":"1.0.0.0", "#.to_string()));

        mock.expect_read_to_string()
            .with(eq(readme_path.clone()))
            .returning(|_| Ok("Old content".to_string()));

        mock.expect_write()
            .with(
                eq(readme_path),
                str::starts_with(format!("## Version {}", expected_ver)),
            )
            .returning(|_, _| Ok(()));

        mock.expect_write()
            .with(
                eq(pkg_path),
                str::contains(format!(r#""version":"{}""#, expected_ver)),
            )
            .returning(|_, _| Ok(()));

        let inst = BumpInstructions {
            author: "Tester".into(),
            project_dir: get_path("root"),
            project_name: "MyProj".into(),
            object: "MyProj".into(),
            description: "Fixing stuff".into(),
            version_original: AppVersion::parse("1.0.0.0").unwrap(),
            is_web_project: true,
        };

        inst.execute_bump(&mock).unwrap();
    }

    #[test]
    fn test_bl_bump_with_multiple_csproj() {
        let mut mock = crate::file_handler::MockFileHandler::new();
        let project_root = get_path("root/MyProj/blMyProj");

        let root_clone = project_root.clone();
        mock.expect_read_dir()
            .with(eq(project_root.clone()))
            .returning(move |_| {
                Ok(vec![
                    // No puedo pasar directorios falsos pq pathbuf.is_dir() siempre va a tirar falso.
                    root_clone.join("blMyProjAPI.csproj"),
                    root_clone.join("blMyProj.csproj"),
                ])
            });

        mock.expect_read_to_string()
            .returning(|_| Ok("<Version>2.1.99.0</Version>".to_string()));

        // Expect the README write
        mock.expect_write()
            .with(eq(project_root.join("README.md")), always())
            .times(1)
            .returning(|_, _| Ok(()));

        // 2.1.99.0 bumped -> 2.2.0.0 (because patch 99 + 1 = 100 triggers minor bump)
        mock.expect_write()
            .with(always(), str::contains("<Version>2.2.0.0</Version>"))
            .times(2)
            .returning(|_, _| Ok(()));

        let inst = BumpInstructions {
            author: "Dev".into(),
            project_dir: get_path("root"),
            project_name: "MyProj".into(),
            object: "MyProj".into(),
            description: "Major patch".into(),
            version_original: AppVersion::parse("2.1.99.0").unwrap(),
            is_web_project: false,
        };

        inst.execute_bump(&mock).unwrap();
    }

    #[test]
    fn test_missing_package_json_writes_nothing() {
        let mut mock = crate::file_handler::MockFileHandler::new();
        mock.expect_read_to_string()
            .returning(|_| Err(anyhow!("Read Fail")));
        mock.expect_write().never();

        let inst = BumpInstructions {
            author: "Tester".into(),
            project_dir: get_path("root"),
            project_name: "MyProj".into(),
            object: "MyProj".into(),
            description: "Fail test".into(),
            version_original: AppVersion::parse("1.0.0.0").unwrap(),
            is_web_project: true,
        };

        assert!(inst.execute_bump(&mock).is_err());
    }

    #[test]
    fn test_failed_version_write_skips_readme() {
        let mut mock = crate::file_handler::MockFileHandler::new();
        let base = get_path("root/MyProj/webMyProj");
        mock.expect_read_to_string()
            .returning(|_| Ok(r#""version":"1.0.0.0""#.to_string()));
        mock.expect_write()
            .with(eq(base.join("package.json")), always())
            .times(1)
            .returning(|_, _| Err(anyhow!("Access Denied")));
        mock.expect_write()
            .with(eq(base.join("README.md")), always())
            .never();

        let inst = BumpInstructions {
            author: "Tester".into(),
            project_dir: get_path("root"),
            project_name: "MyProj".into(),
            object: "MyProj".into(),
            description: "x".into(),
            version_original: AppVersion::parse("1.0.0.0").unwrap(),
            is_web_project: true,
        };

        let err = inst.execute_bump(&mock).unwrap_err();
        assert!(format!("{:#}", err).contains("no se modifico ningun archivo"));
    }

    #[test]
    fn test_failed_second_csproj_restores_first() {
        let mut mock = crate::file_handler::MockFileHandler::new();
        let root = get_path("root/MyProj/blMyProj");
        let first = root.join("a.csproj");
        let second = root.join("b.csproj");
        let (f, s) = (first.clone(), second.clone());
        mock.expect_read_dir()
            .returning(move |_| Ok(vec![f.clone(), s.clone()]));
        mock.expect_read_to_string()
            .returning(|_| Ok("<Version>1.0.0.0</Version>".to_string()));

        let mut seq = mockall::Sequence::new();
        mock.expect_write()
            .with(eq(first.clone()), str::contains("1.0.1.0"))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        mock.expect_write()
            .with(eq(second), always())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Err(anyhow!("locked")));
        // Rollback del primero con su contenido original.
        mock.expect_write()
            .with(eq(first), eq("<Version>1.0.0.0</Version>"))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));

        let inst = BumpInstructions {
            author: "Dev".into(),
            project_dir: get_path("root"),
            project_name: "MyProj".into(),
            object: "MyProj".into(),
            description: "x".into(),
            version_original: AppVersion::parse("1.0.0.0").unwrap(),
            is_web_project: false,
        };

        assert!(inst.execute_bump(&mock).is_err());
    }
}
