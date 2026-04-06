use anyhow::{Context, Result};
use chrono::NaiveDate;
use std::fs;
use std::path::PathBuf;

pub struct BumpMetadata {
    pub version: String,
    pub author: String,
    pub object: String,
    pub date: NaiveDate, // e.g., 2026-04-06
    pub description: String,
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
