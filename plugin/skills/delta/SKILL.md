---
name: delta
description: Subir la versión (bump) de proyectos Fin700 V8 y registrar el cambio en su README usando el CLI de delta. Úsala cuando haya que versionar un proyecto web o bl de Fin700 V8, consultar sus versiones actuales, o listar proyectos de un directorio V8 (p. ej. D:\V8\...).
---

# delta: versionamiento Fin700 V8

`delta.exe` es la herramienta del equipo para versionar proyectos Fin700 V8. **Nunca** edites a
mano `package.json`, `.csproj` ni el `README.md` de versiones: usa el CLI.

## Ubicar el ejecutable
Usa `delta` si está en el PATH. Si no, viene con la extensión de VS Code "Delta Fin700":
`%USERPROFILE%\.vscode\extensions\rafaelmorales.delta-sonda-fin700-<versión>\bin\delta.exe`
(si hay varias versiones, usa la más nueva).

Sin argumentos abre una TUI interactiva: **nunca lo llames sin subcomando**. Antes del primer uso
corre `delta --help`: si no imprime la ayuda (se queda colgado o abre la TUI), el exe es anterior
al CLI; detente y pide al usuario actualizar la extensión.

## Estructura de un proyecto
`<Proyecto>/web<Proyecto>/package.json` (parte web) y `<Proyecto>/bl<Proyecto>/bl<Proyecto>/bl<Proyecto>.csproj` (parte bl).
Versión `8.minor.patch.build`; un bump hace patch+1 (en 100 pasa a minor+1, patch 0).

## Flujo
1. `delta info <PROYECTO> --json` — tipo, versión actual → siguiente de web/bl, archivos que tocaría.
2. `delta bump <PROYECTO> -t web|bl -o <OBJETO> -m "<mensaje>" --dry-run --json` — verifica.
3. Mismo comando sin `--dry-run` para aplicar. Reporta al usuario `from` → `to` y los `files`.

`<PROYECTO>`: ruta a la carpeta del proyecto, o nombre dentro de `default_dir` (`delta config`).

## Reglas
- `-t/--target` es obligatorio si el proyecto tiene web y bl. Si no está claro qué parte cambió
  (según los archivos modificados: front → web, C#/.NET → bl), pregunta al usuario.
- `-o/--object`: el objeto modificado (pantalla, componente, clase, SP...). Por defecto es el nombre del proyecto.
- `-m/--message`: descripción breve en español del cambio. Prefiere pasarlo como argumento
  (`-m "..."`); `-m -` (stdin) solo para multilínea, y desde PowerShell 7, no 5.1 (pierde tildes).
- `-a/--author`: por defecto el `username` del config; no lo cambies salvo que el usuario lo pida.
  Si el CLI responde que no hay usuario configurado, pregúntale al usuario su nombre.
- Si un bump falla (exit 1), no quedó nada a medias: corrige la causa (p. ej. archivo de solo
  lectura o abierto en otro programa) y reintenta.
- Un bump modifica archivos del repo del usuario: confirma con el usuario antes de aplicarlo si
  no lo pidió explícitamente. No hagas commit salvo que te lo pidan.
- Exit code 1 = error (mensaje en stderr), 2 = argumentos inválidos.

Otros: `delta list [DIR] --json` lista proyectos con tipo y versiones. Referencia completa:
`delta --help` y `delta <comando> --help`.
