# delta CLI

`delta.exe` tiene dos modos:

- **Sin argumentos** → abre la TUI (igual que siempre).
- **Con un subcomando** → modo CLI no interactivo, pensado para scripts y agentes.

Errores van a stderr. Exit code: `0` ok, `1` error de ejecución, `2` argumentos inválidos.
Todos los comandos aceptan `--json` para salida estructurada.

## Comandos

```
delta config [--json]
delta list   [DIR] [--json]
delta info   <PROYECTO> [--json]
delta bump   <PROYECTO> -m <MENSAJE|-> [-t web|bl] [-o OBJETO] [-a AUTOR] [--dry-run] [--json]
```

`<PROYECTO>` es la ruta a la carpeta del proyecto (absoluta o relativa al cwd) o, si esa
ruta no existe, el nombre de una carpeta dentro de `default_dir` (ver `delta config`).

| Comando  | Qué hace |
|----------|----------|
| `config` | Muestra `username`, `default_dir` y la ruta del `config.json` (`%APPDATA%\bump\config.json`, compartido con la TUI). |
| `list`   | Lista subcarpetas de `DIR` (por defecto `default_dir`) con tipo (`fullstack`, `web`, `bl`, `git`, `unknown`) y versiones. |
| `info`   | Tipo del proyecto, versión actual → siguiente de web y bl, y archivos que tocaría un bump de cada parte. No escribe nada. |
| `bump`   | Sube la versión (patch +1; en 100 pasa a minor+1, patch 0), antepone una entrada al `README.md` de la parte y actualiza `package.json` (web) o los `.csproj` (bl). |

### Opciones de `bump`

- `-m, --message` (obligatorio): descripción. `-` lee desde stdin (permite multilínea; acepta UTF-8 o Latin-1).
- `-t, --target web|bl`: obligatorio si el proyecto tiene web **y** bl; si tiene solo uno, se infiere.
- `-o, --object`: objeto modificado. Por defecto, el nombre del proyecto.
- `-a, --author`: autor. Por defecto, `username` del config; si no hay usuario configurado, es obligatorio.
- `--dry-run`: no escribe nada; muestra la versión destino y los archivos que se modificarían.

El bump es todo o nada: escribe primero `package.json`/`.csproj` y el `README.md` al final. Si
alguna escritura falla (archivo de solo lectura o bloqueado), restaura lo ya escrito y sale con
exit 1, así que se puede reintentar sin duplicar la entrada del README.

> En Windows PowerShell 5.1, `"texto" | delta ... -m -` envía ASCII y las tildes llegan como `?`.
> Pasa el mensaje como argumento (`-m "Corrección"`) o usa PowerShell 7.

## Flujo recomendado (agentes)

```powershell
delta info MiProyecto --json
delta bump MiProyecto -t web -o grdFactura -m "Corrige total en grilla" --dry-run --json
delta bump MiProyecto -t web -o grdFactura -m "Corrige total en grilla" --json
```

Mensaje multilínea:

```powershell
"Linea 1`nLinea 2" | delta bump MiProyecto -t bl -m -
```

## Salida JSON

`info`:
```json
{
  "name": "MiProyecto",
  "path": "D:\\V8\\Factoring\\MiProyecto",
  "type": "fullstack",
  "web": { "current": "8.1.5.0", "next": "8.1.6.0", "files": ["...\\webMiProyecto\\README.md", "...\\package.json"] },
  "bl":  { "current": "8.1.99.0", "next": "8.2.0.0", "files": ["...\\blMiProyecto\\README.md", "...\\blMiProyecto.csproj"] }
}
```
(`web`/`bl` son `null` si esa parte no existe.)

`bump`:
```json
{
  "project": "MiProyecto", "path": "D:\\V8\\Factoring\\MiProyecto",
  "target": "web", "from": "8.1.5.0", "to": "8.1.6.0",
  "object": "grdFactura", "author": "Rafael Morales",
  "files": ["...\\README.md", "...\\package.json"], "dry_run": false
}
```

`list`: `{ "dir": "...", "projects": [{ "name", "path", "type", "web", "bl" }] }`.
