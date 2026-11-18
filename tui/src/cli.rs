//! Modo CLI no interactivo de delta.
//!
//! `delta` sin argumentos abre la TUI; con un subcomando se ejecuta este modulo. Pensado para
//! scripts y agentes: salida legible por defecto, `--json` para salida estructurada, errores a
//! stderr y exit code distinto de cero cuando algo falla.

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum};
use delta_core::navigation::ProjectType;
use delta_core::{AppVersion, BumpInstructions, FileHandler, NavigationService, OsFileSystem};
use serde_json::{Value, json};

use crate::config::AppConfig;

#[derive(Parser, Debug)]
#[command(
    name = "delta",
    version,
    about = "Versionamiento de proyectos Fin700 V8. Sin argumentos abre la TUI.",
    after_help = "Flujo recomendado: `delta info <PROYECTO>` -> `delta bump ... --dry-run` -> `delta bump ...`"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Muestra la configuracion (usuario, directorio por defecto, ruta del config).
    Config {
        #[arg(long)]
        json: bool,
    },
    /// Lista las subcarpetas de DIR con su tipo de proyecto y versiones.
    List {
        /// Directorio a listar (por defecto: default_dir del config).
        dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Muestra tipo, versiones actuales y siguientes, y archivos que tocaria un bump.
    Info {
        /// Ruta a la carpeta del proyecto, o nombre dentro de default_dir.
        project: String,
        #[arg(long)]
        json: bool,
    },
    /// Sube la version (patch +1) de la parte web o bl del proyecto y agrega entrada al README.
    Bump {
        /// Ruta a la carpeta del proyecto, o nombre dentro de default_dir.
        project: String,
        /// Descripcion del cambio. Usa `-` para leerla desde stdin (permite multilinea).
        #[arg(short, long)]
        message: String,
        /// Parte a versionar. Obligatorio si el proyecto tiene web y bl.
        #[arg(short, long, value_enum)]
        target: Option<Target>,
        /// Objeto modificado (por defecto: nombre del proyecto).
        #[arg(short, long)]
        object: Option<String>,
        /// Autor (por defecto: username del config).
        #[arg(short, long)]
        author: Option<String>,
        /// No escribe nada; muestra lo que haria.
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Web,
    Bl,
}

impl Target {
    fn as_str(self) -> &'static str {
        match self {
            Target::Web => "web",
            Target::Bl => "bl",
        }
    }
}

/// Punto de entrada del modo CLI. Devuelve el exit code (0 ok, 1 error; clap sale con 2 si
/// los argumentos son invalidos).
pub fn run() -> i32 {
    let cli = Cli::parse();
    let fs = OsFileSystem;
    let config = AppConfig::load();

    match execute(cli.command, &fs, &config) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {:#}", e);
            1
        }
    }
}

fn execute(cmd: Command, fs: &OsFileSystem, config: &AppConfig) -> Result<()> {
    match cmd {
        Command::Config { json } => cmd_config(config, json),
        Command::List { dir, json } => cmd_list(fs, config, dir, json),
        Command::Info { project, json } => cmd_info(fs, config, &project, json),
        Command::Bump {
            project,
            message,
            target,
            object,
            author,
            dry_run,
            json,
        } => {
            let message = if message == "-" {
                let mut buf = Vec::new();
                std::io::stdin()
                    .read_to_end(&mut buf)
                    .context("No se pudo leer el mensaje desde stdin")?;
                decode_stdin(buf)
            } else {
                message
            };
            cmd_bump(
                fs,
                config,
                BumpArgs {
                    project,
                    message,
                    target,
                    object,
                    author,
                    dry_run,
                },
                json,
            )
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Proyecto resuelto: carpeta del proyecto, su padre y su nombre.
#[derive(Debug, PartialEq)]
struct ResolvedProject {
    path: PathBuf,
    parent: PathBuf,
    name: String,
}

/// Resuelve `<PROYECTO>`: primero como ruta (absoluta o relativa al cwd), si no, como nombre
/// dentro de `default_dir`.
fn resolve_project(
    fs: &impl FileHandler,
    default_dir: &Path,
    input: &str,
) -> Result<ResolvedProject> {
    let as_path = PathBuf::from(input);
    let candidate = if fs.is_dir(&as_path) {
        as_path
    } else {
        let in_default = default_dir.join(input);
        if fs.is_dir(&in_default) {
            in_default
        } else {
            bail!(
                "Proyecto no encontrado: '{}' (ni como ruta ni dentro de {})",
                input,
                default_dir.display()
            );
        }
    };

    let path = std::path::absolute(&candidate)
        .map(|p| NavigationService::normalize_path(&p))
        .with_context(|| format!("Ruta invalida: {}", candidate.display()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("La ruta no tiene nombre de carpeta: {}", path.display()))?;
    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("La ruta no tiene carpeta padre: {}", path.display()))?;

    Ok(ResolvedProject { path, parent, name })
}

/// Elige la parte a versionar segun lo pedido y lo que exista.
fn choose_target(
    requested: Option<Target>,
    web: Option<AppVersion>,
    bl: Option<AppVersion>,
) -> Result<(Target, AppVersion)> {
    match requested {
        Some(Target::Web) => web.map(|v| (Target::Web, v)).ok_or_else(|| {
            anyhow!("El proyecto no tiene version web legible (web<X>/package.json)")
        }),
        Some(Target::Bl) => bl.map(|v| (Target::Bl, v)).ok_or_else(|| {
            anyhow!("El proyecto no tiene version bl legible (bl<X>/bl<X>/bl<X>.csproj)")
        }),
        None => match (web, bl) {
            (Some(_), Some(_)) => {
                bail!("El proyecto tiene web y bl; indica cual con --target web|bl")
            }
            (Some(v), None) => Ok((Target::Web, v)),
            (None, Some(v)) => Ok((Target::Bl, v)),
            (None, None) => bail!("El proyecto no tiene versiones web ni bl legibles"),
        },
    }
}

/// `AppConfig::default()` usa "Anonymous" cuando no hay config.json.
fn is_default_username(name: &str) -> bool {
    name.trim().is_empty() || name == AppConfig::default().username
}

fn project_type_str(t: &ProjectType) -> &'static str {
    match t {
        ProjectType::FullStack => "fullstack",
        ProjectType::Web => "web",
        ProjectType::BusinessLayer => "bl",
        ProjectType::GitRepository => "git",
        ProjectType::Unknown => "unknown",
    }
}

fn version_json(v: Option<AppVersion>) -> Value {
    match v {
        Some(v) => json!({ "current": v.to_string(), "next": v.bumped().to_string() }),
        None => Value::Null,
    }
}

fn print_json(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

/// Ejecuta (o simula) un bump y devuelve los archivos tocados.
fn run_bump(fs: &impl FileHandler, inst: BumpInstructions, dry_run: bool) -> Result<Vec<PathBuf>> {
    if dry_run {
        Ok(inst.plan_bump(fs)?.into_iter().map(|(p, _)| p).collect())
    } else {
        inst.execute_bump(fs)
    }
}

/// Decodifica el mensaje leido por stdin: UTF-8, o Latin-1 si no es UTF-8 valido.
// shortcut: Latin-1 en vez de cp1252 (difieren solo en 0x80-0x9F: €, comillas tipograficas); usar encoding_rs si aparecen.
fn decode_stdin(bytes: Vec<u8>) -> String {
    let text = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(),
    };
    text.trim_start_matches('\u{feff}').to_string()
}

// ── Comandos ────────────────────────────────────────────────────────────────

fn cmd_config(config: &AppConfig, json: bool) -> Result<()> {
    let path = AppConfig::file_path();
    if json {
        print_json(&json!({
            "username": config.username,
            "default_dir": config.default_dir,
            "config_path": path,
        }));
    } else {
        println!("username:    {}", config.username);
        println!("default_dir: {}", config.default_dir.display());
        println!("config:      {}", path.display());
    }
    Ok(())
}

fn cmd_list(
    fs: &impl FileHandler,
    config: &AppConfig,
    dir: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let dir = dir.unwrap_or_else(|| config.default_dir.clone());
    let dir = std::path::absolute(&dir)
        .map(|p| NavigationService::normalize_path(&p))
        .unwrap_or(dir);
    let names = NavigationService::list_directories(fs, &dir)?;

    let rows: Vec<(String, ProjectType, Option<AppVersion>, Option<AppVersion>)> = names
        .into_iter()
        .filter(|n| !n.starts_with('.'))
        .map(|name| {
            let path = dir.join(&name);
            let ptype = NavigationService::detect_project_type(fs, &path);
            let versions = match ptype {
                ProjectType::FullStack | ProjectType::Web | ProjectType::BusinessLayer => {
                    delta_core::get_current_versions(fs, &path, &name)
                }
                _ => Default::default(),
            };
            (name, ptype, versions.web, versions.bl)
        })
        .collect();

    if json {
        let items: Vec<Value> = rows
            .iter()
            .map(|(name, t, web, bl)| {
                json!({
                    "name": name,
                    "path": dir.join(name),
                    "type": project_type_str(t),
                    "web": web.map(|v| v.to_string()),
                    "bl": bl.map(|v| v.to_string()),
                })
            })
            .collect();
        print_json(&json!({ "dir": dir, "projects": items }));
    } else {
        println!("{}", dir.display());
        let width = rows.iter().map(|r| r.0.len()).max().unwrap_or(0);
        for (name, t, web, bl) in &rows {
            let mut line = format!("  {:<width$}  {:<9}", name, project_type_str(t));
            if let Some(v) = web {
                line.push_str(&format!("  web:{}", v));
            }
            if let Some(v) = bl {
                line.push_str(&format!("  bl:{}", v));
            }
            println!("{}", line.trim_end());
        }
    }
    Ok(())
}

fn cmd_info(fs: &impl FileHandler, config: &AppConfig, project: &str, json: bool) -> Result<()> {
    let p = resolve_project(fs, &config.default_dir, project)?;
    let ptype = NavigationService::detect_project_type(fs, &p.path);
    let versions = delta_core::get_current_versions(fs, &p.path, &p.name);

    // Archivos que tocaria un bump de cada parte (dry-run).
    let files_for = |is_web: bool, v: Option<AppVersion>| -> Vec<PathBuf> {
        let Some(v) = v else { return vec![] };
        let inst = BumpInstructions {
            author: String::new(),
            project_dir: p.parent.clone(),
            project_name: p.name.clone(),
            object: String::new(),
            description: String::new(),
            version_original: v,
            is_web_project: is_web,
        };
        run_bump(fs, inst, true).unwrap_or_default()
    };
    let web_files = files_for(true, versions.web);
    let bl_files = files_for(false, versions.bl);

    if json {
        let mut web = version_json(versions.web);
        if let Value::Object(m) = &mut web {
            m.insert("files".into(), json!(web_files));
        }
        let mut bl = version_json(versions.bl);
        if let Value::Object(m) = &mut bl {
            m.insert("files".into(), json!(bl_files));
        }
        print_json(&json!({
            "name": p.name,
            "path": p.path,
            "type": project_type_str(&ptype),
            "web": web,
            "bl": bl,
        }));
    } else {
        println!("Proyecto: {}", p.name);
        println!("Ruta:     {}", p.path.display());
        println!("Tipo:     {}", project_type_str(&ptype));
        for (label, v, files) in [
            ("web", versions.web, &web_files),
            ("bl", versions.bl, &bl_files),
        ] {
            match v {
                Some(v) => {
                    println!("{:<4} {} -> {}", label, v, v.bumped());
                    for f in files {
                        println!("       {}", f.display());
                    }
                }
                None => println!("{:<4} -", label),
            }
        }
    }
    Ok(())
}

struct BumpArgs {
    project: String,
    message: String,
    target: Option<Target>,
    object: Option<String>,
    author: Option<String>,
    dry_run: bool,
}

fn cmd_bump(fs: &impl FileHandler, config: &AppConfig, args: BumpArgs, json: bool) -> Result<()> {
    // Normaliza fin de linea y quita espacios sobrantes al final.
    let message = args.message.replace("\r\n", "\n").trim_end().to_string();
    if message.trim().is_empty() {
        bail!("El mensaje (--message) no puede estar vacio");
    }

    let p = resolve_project(fs, &config.default_dir, &args.project)?;
    let versions = delta_core::get_current_versions(fs, &p.path, &p.name);
    let (target, current) = choose_target(args.target, versions.web, versions.bl)?;
    let next = current.bumped();

    let object = args
        .object
        .filter(|o| !o.trim().is_empty())
        .unwrap_or_else(|| p.name.clone());
    let author = match args.author.filter(|a| !a.trim().is_empty()) {
        Some(a) => a,
        None if is_default_username(&config.username) => bail!(
            "No hay usuario configurado ({}); usa --author o configuralo en la TUI",
            AppConfig::file_path().display()
        ),
        None => config.username.clone(),
    };

    let inst = BumpInstructions {
        author: author.clone(),
        project_dir: p.parent.clone(),
        project_name: p.name.clone(),
        object: object.clone(),
        description: message,
        version_original: current,
        is_web_project: target == Target::Web,
    };
    let files = run_bump(fs, inst, args.dry_run)?;

    if json {
        print_json(&json!({
            "project": p.name,
            "path": p.path,
            "target": target.as_str(),
            "from": current.to_string(),
            "to": next.to_string(),
            "object": object,
            "author": author,
            "files": files,
            "dry_run": args.dry_run,
        }));
    } else {
        let prefix = if args.dry_run { "[dry-run] " } else { "" };
        println!(
            "{}{} {}: {} -> {}",
            prefix,
            p.name,
            target.as_str(),
            current,
            next
        );
        println!("  objeto: {}", object);
        println!("  autor:  {}", author);
        for f in &files {
            println!("  {} {}", if args.dry_run { "~" } else { "M" }, f.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use delta_core::file_handler::MockFileHandler;

    fn v(s: &str) -> Option<AppVersion> {
        Some(AppVersion::parse(s).unwrap())
    }

    #[test]
    fn choose_target_infers_single_side() {
        let (t, ver) = choose_target(None, v("8.1.2.0"), None).unwrap();
        assert_eq!(t, Target::Web);
        assert_eq!(ver.to_string(), "8.1.2.0");
        let (t, _) = choose_target(None, None, v("8.1.2.0")).unwrap();
        assert_eq!(t, Target::Bl);
    }

    #[test]
    fn choose_target_requires_flag_for_fullstack() {
        assert!(choose_target(None, v("8.1.2.0"), v("8.1.3.0")).is_err());
        let (t, ver) = choose_target(Some(Target::Bl), v("8.1.2.0"), v("8.1.3.0")).unwrap();
        assert_eq!(t, Target::Bl);
        assert_eq!(ver.to_string(), "8.1.3.0");
    }

    #[test]
    fn choose_target_errors_when_side_missing() {
        assert!(choose_target(Some(Target::Web), None, v("8.1.2.0")).is_err());
        assert!(choose_target(None, None, None).is_err());
    }

    #[test]
    fn resolve_project_falls_back_to_default_dir() {
        let mut mock = MockFileHandler::new();
        let default_dir = std::path::absolute("V8").unwrap();
        let expected = default_dir.join("MyProj");
        let exp = expected.clone();
        mock.expect_is_dir().returning(move |p| p == exp.as_path());

        let r = resolve_project(&mock, &default_dir, "MyProj").unwrap();
        assert_eq!(r.path, expected);
        assert_eq!(r.parent, default_dir);
        assert_eq!(r.name, "MyProj");
    }

    #[test]
    fn decode_stdin_accepts_utf8_latin1_and_bom() {
        assert_eq!(decode_stdin("Corrección".as_bytes().to_vec()), "Corrección");
        assert_eq!(decode_stdin(b"Correcci\xf3n".to_vec()), "Corrección");
        assert_eq!(decode_stdin("\u{feff}ñandú".as_bytes().to_vec()), "ñandú");
    }

    #[test]
    fn default_username_is_rejected() {
        assert!(is_default_username("Anonymous"));
        assert!(is_default_username("  "));
        assert!(!is_default_username("Rafael Morales"));
    }

    #[test]
    fn resolve_project_errors_when_missing() {
        let mut mock = MockFileHandler::new();
        mock.expect_is_dir().returning(|_| false);
        assert!(resolve_project(&mock, Path::new("V8"), "Nope").is_err());
    }
}
