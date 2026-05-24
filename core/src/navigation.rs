use crate::FileHandler;
use anyhow::{Result, anyhow};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NavigationError {
    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Not a directory: {0}")]
    NotADirectory(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Already at root directory")]
    AtRootDirectory,

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Directory,
    File,
    Symlink,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_type: FileType,
    pub is_hidden: bool,
    pub is_accessible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuzzyMatch {
    pub text: String,
    pub match_indices: Vec<usize>,
    pub score: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Web,
    BusinessLayer,
    FullStack, // Has both web and bl
    GitRepository,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DirectoryDisplayInfo {
    pub project_type: ProjectType,
    pub versions: Option<String>,
}

pub struct NavigationService;

// Core Path Operations
impl NavigationService {
    /// Normalizes a path by resolving relative components and cleaning up separators
    pub fn normalize_path(path: &Path) -> PathBuf {
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::CurDir => {
                    // Skip
                }
                other => {
                    components.push(other);
                }
            }
        }

        components.iter().collect()
    }

    /// Returns the parent directory if it exists
    pub fn parent_directory(path: &Path) -> Option<PathBuf> {
        path.parent().map(|p| p.to_path_buf())
    }

    /// Joins a base path with a relative child, handling edge cases
    pub fn join_paths(base: &Path, child: &str) -> Result<PathBuf> {
        if child.contains("..") || child.contains('/') || child.contains('\\') {
            return Err(anyhow!("Invalid directory name: {}", child));
        }

        let joined = base.join(child);

        // Basic security check to prevent directory traversal
        let canonical_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
        let canonical_joined = joined.canonicalize().unwrap_or_else(|_| joined.clone());

        if !canonical_joined.starts_with(&canonical_base) {
            return Err(anyhow!("Path traversal attempt detected"));
        }

        Ok(joined)
    }

    /// Checks if we're at the filesystem root
    pub fn is_root(path: &Path) -> bool {
        path.parent().is_none()
    }

    /// Resolves a potentially relative path against a base
    pub fn resolve_path(base: &Path, relative: &Path) -> PathBuf {
        if relative.is_absolute() {
            relative.to_path_buf()
        } else {
            base.join(relative)
        }
    }
}

// Directory Listing and Discovery
impl NavigationService {
    /// Lists all entries in a directory with metadata
    pub fn list_directory(fs: &impl FileHandler, path: &Path) -> Result<Vec<DirectoryEntry>> {
        if !fs.exists(path) {
            return Err(NavigationError::DirectoryNotFound(path.display().to_string()).into());
        }

        if !fs.is_dir(path) {
            return Err(NavigationError::NotADirectory(path.display().to_string()).into());
        }

        let entries = fs
            .read_dir(path)
            .map_err(|_| NavigationError::PermissionDenied(path.display().to_string()))?;

        let mut result = Vec::new();

        for entry_path in entries {
            let name = fs
                .file_name(&entry_path)
                .unwrap_or_else(|| entry_path.to_string_lossy().to_string());

            let is_hidden = name.starts_with('.');
            let file_type = if fs.is_dir(&entry_path) {
                FileType::Directory
            } else if fs.is_file(&entry_path) {
                FileType::File
            } else if entry_path.is_symlink() {
                FileType::Symlink
            } else {
                FileType::Unknown
            };

            // Check if we can actually access this entry
            let is_accessible = fs.read_dir(&entry_path).is_ok() || fs.is_file(&entry_path);

            result.push(DirectoryEntry {
                name,
                path: entry_path,
                file_type,
                is_hidden,
                is_accessible,
            });
        }

        // Sort: directories first, then alphabetical
        result.sort_by(|a, b| {
            let a_is_dir = a.file_type == FileType::Directory;
            let b_is_dir = b.file_type == FileType::Directory;

            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(result)
    }

    /// Lists only directories, sorted alphabetically
    pub fn list_directories(fs: &impl FileHandler, path: &Path) -> Result<Vec<String>> {
        let entries = Self::list_directory(fs, path)?;

        Ok(entries
            .into_iter()
            .filter(|e| e.file_type == FileType::Directory && e.is_accessible)
            .map(|e| e.name)
            .collect())
    }

    /// Finds directories matching a specific pattern or criteria
    pub fn find_directories(
        fs: &impl FileHandler,
        base_path: &Path,
        predicate: impl Fn(&DirectoryEntry) -> bool,
    ) -> Result<Vec<PathBuf>> {
        let entries = Self::list_directory(fs, base_path)?;

        Ok(entries
            .into_iter()
            .filter(|e| e.file_type == FileType::Directory && predicate(e))
            .map(|e| e.path)
            .collect())
    }

    /// Builds a breadcrumb trail from root to the current path
    pub fn build_breadcrumbs(path: &Path) -> Vec<PathBuf> {
        let mut breadcrumbs = Vec::new();
        let mut current = path.to_path_buf();

        breadcrumbs.push(current.clone());

        while let Some(parent) = current.parent() {
            if parent.as_os_str().is_empty() {
                break;
            }
            breadcrumbs.push(parent.to_path_buf());
            current = parent.to_path_buf();
        }

        breadcrumbs.reverse();
        breadcrumbs
    }
}

// Fuzzy Search and Filtering
impl NavigationService {
    /// Performs fuzzy matching on a list of strings
    pub fn fuzzy_filter(items: &[String], query: &str, case_sensitive: bool) -> Vec<FuzzyMatch> {
        if query.is_empty() {
            return items
                .iter()
                .map(|item| FuzzyMatch {
                    text: item.clone(),
                    match_indices: vec![],
                    score: 0,
                })
                .collect();
        }

        let matcher = SkimMatcherV2::default().smart_case();
        let query = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let mut results: Vec<FuzzyMatch> = items
            .iter()
            .filter_map(|item| {
                let search_text = if case_sensitive {
                    item.clone()
                } else {
                    item.to_lowercase()
                };

                matcher
                    .fuzzy_indices(&search_text, &query)
                    .map(|(score, indices)| FuzzyMatch {
                        text: item.clone(),
                        match_indices: indices,
                        score,
                    })
            })
            .collect();

        // Sort by score (highest first), then alphabetically
        results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));

        results
    }

    /// Filters directory entries using fuzzy matching
    pub fn fuzzy_filter_entries(entries: &[DirectoryEntry], query: &str) -> Vec<FuzzyMatch> {
        let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
        Self::fuzzy_filter(&names, query, false)
    }

    /// Quick search that returns exact matches first, then fuzzy matches
    pub fn smart_search(items: &[String], query: &str) -> Vec<FuzzyMatch> {
        if query.is_empty() {
            return Self::fuzzy_filter(items, query, false);
        }

        let mut results = Self::fuzzy_filter(items, query, false);

        // Sort exact matches to the top
        results.sort_by_key(|m| {
            if m.text.to_lowercase() == query.to_lowercase() {
                0
            } else if m.text.to_lowercase().starts_with(&query.to_lowercase()) {
                1
            } else {
                2
            }
        });

        results
    }

    /// Highlights matching characters in a fuzzy match result
    pub fn highlight_matches(
        text: &str,
        indices: &[usize],
        highlight_start: &str,
        highlight_end: &str,
    ) -> String {
        if indices.is_empty() {
            return text.to_string();
        }

        let mut result = String::new();
        let mut last_idx = 0;

        for &idx in indices {
            if idx > last_idx {
                result.push_str(&text[last_idx..idx]);
            }
            result.push_str(highlight_start);
            result.push(text.chars().nth(idx).unwrap_or(' '));
            result.push_str(highlight_end);
            last_idx = idx + 1;
        }

        if last_idx < text.len() {
            result.push_str(&text[last_idx..]);
        }

        result
    }
}

// Project Structure Discovery
impl NavigationService {
    /// Checks if a directory might be a project root (has expected structure)
    pub fn is_project_directory(fs: &impl FileHandler, path: &Path) -> bool {
        if !fs.is_dir(path) {
            return false;
        }

        // Check for common project indicators
        let has_git = fs.exists(&path.join(".git"));
        let has_readme = fs.exists(&path.join("README.md")) || fs.exists(&path.join("readme.md"));
        let has_package_json = fs.exists(&path.join("package.json"));
        let has_csproj = Self::find_csproj_files(fs, path).is_ok()
            && !Self::find_csproj_files(fs, path).unwrap().is_empty();

        has_git || has_readme || has_package_json || has_csproj
    }

    /// Finds all project directories from a starting point (non-recursive by default)
    pub fn discover_projects(
        fs: &impl FileHandler,
        root_path: &Path,
        recursive: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut projects = Vec::new();
        let entries = Self::list_directory(fs, root_path)?;

        for entry in &entries {
            if entry.file_type != FileType::Directory {
                continue;
            }

            if entry.is_hidden {
                continue;
            }

            if Self::is_project_directory(fs, &entry.path) {
                projects.push(entry.path.clone());
            } else if recursive {
                match Self::discover_projects(fs, &entry.path, recursive) {
                    Ok(mut child_projects) => projects.append(&mut child_projects),
                    Err(_) => continue, // Skip inaccessible directories
                }
            }
        }

        Ok(projects)
    }

    /// Finds .csproj files in a directory (shallow search as per your existing logic)
    pub fn find_csproj_files(fs: &impl FileHandler, base_path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let entries = fs
            .read_dir(base_path)
            .map_err(|_| anyhow!("Cannot read directory: {}", base_path.display()))?;

        for path in entries {
            if fs.is_dir(&path) {
                if let Ok(sub_entries) = fs.read_dir(&path) {
                    for sub_path in sub_entries {
                        if Self::is_csproj_file(&sub_path) {
                            files.push(sub_path);
                        }
                    }
                }
            } else if Self::is_csproj_file(&path) {
                files.push(path);
            }
        }

        Ok(files)
    }

    fn is_csproj_file(path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("csproj"))
            .unwrap_or(false)
    }
}

// Path validation and security
impl NavigationService {
    /// Validates that a path is safe to navigate to
    pub fn validate_navigation_path(fs: &impl FileHandler, path: &Path) -> Result<()> {
        if !fs.exists(path) {
            return Err(NavigationError::DirectoryNotFound(path.display().to_string()).into());
        }

        if !fs.is_dir(path) {
            return Err(NavigationError::NotADirectory(path.display().to_string()).into());
        }

        // Check permissions by trying to read the directory
        fs.read_dir(path)
            .map_err(|_| NavigationError::PermissionDenied(path.display().to_string()))?;

        Ok(())
    }

    /// Sanitizes user input for path components
    pub fn sanitize_path_component(component: &str) -> Result<String> {
        if component.is_empty() {
            return Err(anyhow!("Empty path component"));
        }

        if component.contains("..") {
            return Err(anyhow!("Path traversal not allowed"));
        }

        if component.contains('/') || component.contains('\\') {
            return Err(anyhow!("Invalid characters in path component"));
        }

        // Additional security checks
        for c in component.chars() {
            if c.is_control() {
                return Err(anyhow!("Control characters not allowed"));
            }
        }

        Ok(component.to_string())
    }

    /// Detects the type of project in a directory based on its structure
    pub fn detect_project_type(fs: &impl FileHandler, path: &Path) -> ProjectType {
        if !fs.is_dir(path) {
            return ProjectType::Unknown;
        }

        let dir_name = fs.file_name(path).unwrap_or_default();

        // Check for Business Layer structure: {ProjectName}/bl{ProjectName}/bl{ProjectName}/*.csproj
        let bl_path = path.join(format!("bl{}", dir_name));
        let has_bl = fs.is_dir(&bl_path) && fs.is_dir(&bl_path.join(format!("bl{}", dir_name)));

        // Check for Web structure: {ProjectName}/web{ProjectName}/package.json
        let web_path = path.join(format!("web{}", dir_name));
        let has_web = fs.is_dir(&web_path) && fs.exists(&web_path.join("package.json"));

        match (has_web, has_bl) {
            (true, true) => ProjectType::FullStack,
            (true, false) => ProjectType::Web,
            (false, true) => ProjectType::BusinessLayer,
            (false, false) => {
                // Check for .git directory (git repository root)
                if fs.exists(&path.join(".git")) {
                    ProjectType::GitRepository
                } else {
                    ProjectType::Unknown
                }
            }
        }
    }

    /// Gets display information for a directory based on its project type
    pub fn get_directory_display_info(fs: &impl FileHandler, path: &Path) -> DirectoryDisplayInfo {
        let project_type = Self::detect_project_type(fs, path);
        let versions = if project_type != ProjectType::Unknown {
            Self::get_project_versions(fs, path)
        } else {
            None
        };

        DirectoryDisplayInfo {
            project_type,
            versions,
        }
    }

    /// Gets project versions if available
    fn get_project_versions(fs: &impl FileHandler, path: &Path) -> Option<String> {
        let dir_name = fs.file_name(path)?;

        let mut parts = Vec::new();

        // Check for web version from package.json
        let web_package_json = path.join(format!("web{}/package.json", dir_name));
        if fs.exists(&web_package_json) {
            if let Some(version) = crate::extract_version_from_package_json(fs, &web_package_json) {
                parts.push(format!("web:{}", version));
            }
        }

        // Check for BL version from .csproj file
        let bl_csproj = path.join(format!(
            "bl{}/bl{}/bl{}.csproj",
            dir_name, dir_name, dir_name
        ));
        if fs.exists(&bl_csproj) {
            if let Some(version) = crate::extract_version_from_csproj(fs, &bl_csproj) {
                parts.push(format!("bl:{}", version));
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockFileHandler;
    use std::path::PathBuf;

    fn setup_mock_fs() -> MockFileHandler {
        MockFileHandler::new()
    }

    #[test]
    fn test_list_directories_with_mock() {
        let mut mock = setup_mock_fs();

        // Setup test data
        let test_path = PathBuf::from("/test");
        mock.expect_exists()
            .with(mockall::predicate::eq(test_path.clone()))
            .returning(|_| true);

        mock.expect_is_dir()
            .with(mockall::predicate::eq(test_path.clone()))
            .returning(|_| true);

        mock.expect_read_dir()
            .with(mockall::predicate::eq(test_path.clone()))
            .returning(|_| {
                Ok(vec![
                    PathBuf::from("/test/dir1"),
                    PathBuf::from("/test/file.txt"),
                    PathBuf::from("/test/dir2"),
                ])
            });

        mock.expect_file_name()
            .with(mockall::predicate::eq(PathBuf::from("/test/dir1")))
            .returning(|_| Some("dir1".to_string()));

        mock.expect_file_name()
            .with(mockall::predicate::eq(PathBuf::from("/test/file.txt")))
            .returning(|_| Some("file.txt".to_string()));

        mock.expect_file_name()
            .with(mockall::predicate::eq(PathBuf::from("/test/dir2")))
            .returning(|_| Some("dir2".to_string()));

        // Setup directory checks
        mock.expect_is_dir()
            .returning(|p| !p.to_string_lossy().contains(".txt"));

        mock.expect_is_file()
            .returning(|p| p.to_string_lossy().contains(".txt"));

        mock.expect_read_dir() // For accessibility check
            .returning(|_| Ok(vec![]));

        let result = NavigationService::list_directories(&mock, &test_path);
        assert!(result.is_ok());

        let dirs = result.unwrap();
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0], "dir1");
        assert_eq!(dirs[1], "dir2");
    }

    #[test]
    fn test_fuzzy_filter_empty_query() {
        let items = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];

        let result = NavigationService::fuzzy_filter(&items, "", false);
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|m| m.match_indices.is_empty()));
    }

    #[test]
    fn test_fuzzy_filter_with_query() {
        let items = vec![
            "alpha_project".to_string(),
            "alpha_test".to_string(),
            "beta".to_string(),
        ];

        let result = NavigationService::fuzzy_filter(&items, "alp", false);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "alpha_project");
    }

    #[test]
    fn test_smart_search_prioritizes_exact_matches() {
        let items = vec![
            "alpha_something".to_string(),
            "alpha".to_string(),
            "alpha_else".to_string(),
        ];

        let result = NavigationService::smart_search(&items, "alpha");
        assert!(!result.is_empty());
        assert_eq!(result[0].text, "alpha"); // Exact match first
    }

    #[test]
    fn test_parent_directory() {
        let path = PathBuf::from("/home/user/documents");
        let parent = NavigationService::parent_directory(&path);
        assert_eq!(parent, Some(PathBuf::from("/home/user")));
    }

    #[test]
    fn test_parent_directory_at_root() {
        let path = PathBuf::from("/");
        let parent = NavigationService::parent_directory(&path);
        assert_eq!(parent, Some(PathBuf::from("/")));
    }

    #[test]
    fn test_is_root() {
        assert!(NavigationService::is_root(Path::new("/")));
        assert!(!NavigationService::is_root(Path::new("/home")));
    }

    #[test]
    fn test_join_paths_safe() {
        let base = PathBuf::from("/home/user");
        let result = NavigationService::join_paths(&base, "documents");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/home/user/documents"));
    }

    #[test]
    fn test_join_paths_prevents_traversal() {
        let base = PathBuf::from("/home/user");
        let result = NavigationService::join_paths(&base, "../etc");
        assert!(result.is_err());
    }

    #[test]
    fn test_highlight_matches() {
        let text = "hello world";
        let indices = vec![0, 1, 6];
        let result = NavigationService::highlight_matches(text, &indices, "<b>", "</b>");
        assert_eq!(result, "<b>h</b><b>e</b>llo <b>w</b>orld");
    }

    #[test]
    fn test_build_breadcrumbs() {
        let path = PathBuf::from("/home/user/documents");
        let breadcrumbs = NavigationService::build_breadcrumbs(&path);

        assert_eq!(breadcrumbs.len(), 3);
        assert_eq!(breadcrumbs[0], PathBuf::from("/home"));
        assert_eq!(breadcrumbs[1], PathBuf::from("/home/user"));
        assert_eq!(breadcrumbs[2], PathBuf::from("/home/user/documents"));
    }

    #[test]
    fn test_find_directories_with_predicate() {
        let mut mock = setup_mock_fs();
        let test_path = PathBuf::from("/test");

        mock.expect_exists().returning(|_| true);
        mock.expect_is_dir().returning(|_| true);
        mock.expect_is_file().returning(|_| false);
        mock.expect_read_dir().returning(|_| {
            Ok(vec![
                PathBuf::from("/test/project-1"),
                PathBuf::from("/test/project-2"),
                PathBuf::from("/test/other"),
            ])
        });

        mock.expect_file_name()
            .returning(|p| Some(p.file_name().unwrap().to_string_lossy().to_string()));

        let result = NavigationService::find_directories(&mock, &test_path, |e| {
            e.name.starts_with("project")
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_validate_navigation_path_success() {
        let mut mock = setup_mock_fs();
        let path = PathBuf::from("/valid/dir");

        mock.expect_exists()
            .with(mockall::predicate::eq(path.clone()))
            .returning(|_| true);

        mock.expect_is_dir()
            .with(mockall::predicate::eq(path.clone()))
            .returning(|_| true);

        mock.expect_read_dir()
            .with(mockall::predicate::eq(path.clone()))
            .returning(|_| Ok(vec![]));

        let result = NavigationService::validate_navigation_path(&mock, &path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_navigation_path_not_found() {
        let mut mock = setup_mock_fs();
        let path = PathBuf::from("/nonexistent");

        mock.expect_exists()
            .with(mockall::predicate::eq(path.clone()))
            .returning(|_| false);

        let result = NavigationService::validate_navigation_path(&mock, &path);
        assert!(result.is_err());
    }
}
