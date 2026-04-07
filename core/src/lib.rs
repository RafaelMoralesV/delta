use anyhow::anyhow;
use anyhow::{Context, Result};
use chrono::NaiveDate;
use regex::Regex;
use std::path::PathBuf;
use std::{fmt, fs};

pub struct BumpMetadata {
    pub version: String,
    pub author: String,
    pub object: String,
    pub date: NaiveDate, // e.g., 2026-04-06
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: u32,
}

pub enum ProjectType {
    Web(PathBuf), // Path to web{{Project Name}}
    Bl(PathBuf),  // Path to bl{{Project Name}}
}

impl BumpMetadata {
    /// Formats the metadata into the markdown string required for README.md
    pub fn to_markdown_entry(&self) -> String {
        format!(
            "## Version {}\n\n\
            - Nombre: {}\n\
            - Objeto: {}\n\
            - Fecha: {}\n\
            - Descripción: {}\n\n",
            self.version,
            self.author,
            self.object,
            self.date.format("%d/%m/%Y"),
            self.description
        )
    }
}

pub fn bump_project(project: ProjectType, metadata: &BumpMetadata) -> Result<()> {
    match project {
        ProjectType::Web(path) => {
            prepend_readme(&path.join("README.md"), metadata)?;
            bump_package_json(&path.join("package.json"), &metadata.version)?;
        }
        ProjectType::Bl(path) => {
            prepend_readme(&path.join("README.md"), metadata)?;
            // Find all .csproj files in the bl directory tree
            let csproj_files = find_csproj_files(&path)?;
            for csproj in csproj_files {
                bump_csproj(&csproj, &metadata.version)?;
            }
        }
    }
    Ok(())
}

fn prepend_readme(path: &PathBuf, metadata: &BumpMetadata) -> Result<()> {
    let existing_content = fs::read_to_string(path).unwrap_or_default();
    let new_entry = metadata.to_markdown_entry();

    // Simplest approach: just slap it at the top.
    // If you have a `# Title`, you'd use regex to insert *after* the title.
    let updated_content = format!("{}{}", new_entry, existing_content);
    fs::write(path, updated_content)?;
    Ok(())
}

fn bump_package_json(path: &PathBuf, new_version: &str) -> Result<()> {
    let content = fs::read_to_string(path).context("Failed to read package.json")?;

    // Using Regex to preserve file formatting and potential pseudo-JSON comments
    let re = regex::Regex::new(r#"("version"\s*:\s*)"[^"]+""#).unwrap();
    let updated = re.replace(&content, format!(r#"${{1}}"{new_version}""#));

    fs::write(path, updated.as_bytes())?;
    Ok(())
}

fn bump_csproj(path: &PathBuf, new_version: &str) -> Result<()> {
    let content = fs::read_to_string(path).context("Failed to read .csproj")?;

    // Regex targets the <Version> tag
    let re = regex::Regex::new(r"(?i)<Version>.*?</Version>").unwrap();
    let updated = re.replace(&content, format!("<Version>{}</Version>", new_version));

    fs::write(path, updated.as_bytes())?;
    Ok(())
}

fn find_csproj_files(base_path: &PathBuf) -> Result<Vec<PathBuf>> {
    // You would use the `walkdir` crate here in a real implementation
    // to recursively find all files ending in .csproj
    Ok(vec![])
}

impl AppVersion {
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return Err(anyhow!("Version must follow X.Y.Z.W format"));
        }
        Ok(Self {
            major: parts[0].parse()?,
            minor: parts[1].parse()?,
            patch: parts[2].parse()?,
            build: parts[3].parse()?,
        })
    }

    pub fn bumped(&self) -> Self {
        let mut next = *self;
        if next.patch >= 99 {
            next.minor += 1;
            next.patch = 0;
        } else {
            next.patch += 1;
        }
        next
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

pub struct ProjectVersions {
    pub web: Option<AppVersion>,
    pub bl: Option<AppVersion>,
}

/// Attempts to read the current versions for a selected project directory
pub fn get_current_versions(project_dir: &PathBuf, project_name: &str) -> ProjectVersions {
    let web_path = project_dir
        .join(format!("web{}", project_name))
        .join("package.json");
    let bl_path = project_dir
        .join(format!("bl{}", project_name))
        .join(format!("bl{}", project_name))
        .join(format!("bl{}.csproj", project_name)); // Checks the base csproj as source of truth

    ProjectVersions {
        web: extract_version_from_package_json(&web_path),
        bl: extract_version_from_csproj(&bl_path),
    }
}

fn extract_version_from_package_json(path: &PathBuf) -> Option<AppVersion> {
    let content = fs::read_to_string(path).ok()?;
    let re = Regex::new(r#"("version"\s*:\s*)"([^"]+)""#).ok()?;
    let caps = re.captures(&content)?;
    AppVersion::parse(caps.get(2)?.as_str()).ok()
}

fn extract_version_from_csproj(path: &PathBuf) -> Option<AppVersion> {
    let content = fs::read_to_string(path).ok()?;
    let re = Regex::new(r"(?i)<Version>(.*?)</Version>").ok()?;
    let caps = re.captures(&content)?;
    AppVersion::parse(caps.get(1)?.as_str()).ok()
}
