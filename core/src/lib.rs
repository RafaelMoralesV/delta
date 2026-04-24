use anyhow::Context;
use anyhow::{Result, anyhow};
use chrono::Local;
use regex::Regex;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: u32,
}

impl AppVersion {
    pub fn bumped(&self) -> Self {
        let mut next = *self;
        next.patch += 1;
        if next.patch >= 100 {
            next.minor += 1;
            next.patch = 0;
        }
        next
    }
}

pub struct ProjectVersions {
    pub web: Option<AppVersion>,
    pub bl: Option<AppVersion>,
}

impl AppVersion {
    /// Parses a version string in X.Y.Z.W format
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

/// Attempts to read the current versions for a selected project directory
pub fn get_current_versions(project_dir: &Path, project_name: &str) -> ProjectVersions {
    let web_path = project_dir
        .join(format!("web{}", project_name))
        .join("package.json");

    // Note: Adjusted to match your specific bl structure logic
    let bl_path = project_dir
        .join(format!("bl{}", project_name))
        .join(format!("bl{}", project_name))
        .join(format!("bl{}.csproj", project_name));

    ProjectVersions {
        web: extract_version_from_package_json(&web_path),
        bl: extract_version_from_csproj(&bl_path),
    }
}

fn extract_version_from_package_json(path: &PathBuf) -> Option<AppVersion> {
    let content = fs::read_to_string(path).ok()?;
    let re = Regex::new(r#""version"\s*:\s*"([^"]+)""#).ok()?;
    let caps = re.captures(&content)?;
    AppVersion::parse(caps.get(1)?.as_str()).ok()
}

fn extract_version_from_csproj(path: &PathBuf) -> Option<AppVersion> {
    let content = fs::read_to_string(path).ok()?;
    let re = Regex::new(r"(?i)<Version>(.*?)</Version>").ok()?;
    let caps = re.captures(&content)?;
    AppVersion::parse(caps.get(1)?.as_str()).ok()
}

#[derive(Debug)]
pub struct BumpInstructions {
    pub author: String,
    pub project_dir: PathBuf,
    pub project_name: String,
    pub description: String,
}

pub fn execute_bump(
    instructions: BumpInstructions,
    current_version: AppVersion,
    is_web: bool,
) -> Result<()> {
    let target_version = current_version.bumped();
    let date_str = Local::now().format("%d/%m/%Y").to_string();

    // Determine the "Objeto" name (e.g., "webProjectName" or "blProjectName")
    let prefix = if is_web { "web" } else { "bl" };
    let object_name = format!("{}{}", prefix, instructions.project_name);
    let project_path = instructions
        .project_dir
        .join(&instructions.project_name)
        .join(&object_name);

    update_readme(
        &project_path.join("README.md"),
        &target_version.to_string(),
        &instructions.author,
        &object_name,
        &date_str,
        &instructions.description,
    )?;

    if is_web {
        update_package_json(
            &project_path.join("package.json"),
            &target_version.to_string(),
        )?;
    } else {
        let csproj_files = find_csproj_shallow(&project_path);
        for path in csproj_files {
            update_csproj(&path, &target_version.to_string())?;
        }
    }

    Ok(())
}

fn update_readme(
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
        - Descripción: {}\n\n",
        ver, aut, obj, date, desc
    );

    let existing = fs::read_to_string(path).unwrap_or_default();
    fs::write(path, format!("{}{}", entry, existing))
        .with_context(|| format!("Failed to update README at {:?}", path))
}

fn update_package_json(path: &Path, version: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let re = Regex::new(r#"("version"\s*:\s*)"[^"]+""#).unwrap();
    let updated = re.replace(&content, format!(r#"$1"{version}""#));
    fs::write(path, updated.as_bytes())?;
    Ok(())
}

fn update_csproj(path: &Path, version: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let re = Regex::new(r"(?i)<Version>.*?</Version>").unwrap();
    let updated = re.replace(&content, format!("<Version>{}</Version>", version));
    fs::write(path, updated.as_bytes())?;
    Ok(())
}

fn find_csproj_shallow(base_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let Ok(entries) = fs::read_dir(base_path) else {
        return files;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            // Check inside immediate subdirectories (e.g., blProjectAPI/)
            if let Ok(sub_entries) = fs::read_dir(&path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    if is_csproj(&sub_path) {
                        files.push(sub_path);
                    }
                }
            }
        } else if is_csproj(&path) {
            files.push(path);
        }
    }
    files
}

fn is_csproj(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("csproj")
}
