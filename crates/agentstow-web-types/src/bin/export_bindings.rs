use std::path::{Path, PathBuf};

fn print_help() {
    eprintln!(
        r#"Export agentstow-web-types bindings (ts-rs) into a TypeScript directory.

Usage:
  cargo run -p agentstow-web-types --bin export_bindings -- [--out <dir>] [--no-index]

Options:
  --out <dir>    Output directory (default: web/src/lib/bindings)
  --no-index     Do not write bindings/index.ts
  -h, --help     Print this help
"#
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out_dir: Option<PathBuf> = None;
    let mut write_index = true;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                let Some(value) = args.next() else {
                    print_help();
                    return Err("--out requires a value".into());
                };
                out_dir = Some(PathBuf::from(value));
            }
            "--no-index" => {
                write_index = false;
            }
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            other => {
                if out_dir.is_none() {
                    out_dir = Some(PathBuf::from(other));
                } else {
                    print_help();
                    return Err(format!("unexpected argument: {other}").into());
                }
            }
        }
    }

    let out_dir = out_dir.unwrap_or_else(|| PathBuf::from("web/src/lib/bindings"));
    std::fs::create_dir_all(&out_dir)?;

    // ts-rs reads `TS_RS_EXPORT_DIR` via `Config::from_env()`.
    // SAFETY: This is a short-lived CLI process. We set the env var before any
    // multi-threaded work begins (ts-rs export is single-threaded here).
    unsafe {
        std::env::set_var("TS_RS_EXPORT_DIR", &out_dir);
    }
    agentstow_web_types::export_bindings()?;

    if write_index {
        write_bindings_index(&out_dir)?;
    }

    Ok(())
}

fn write_bindings_index(out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut names: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(out_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ts") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if file_name == "index.ts" {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        names.push(stem.to_string());
    }
    names.sort();

    let mut out = String::new();
    for name in names {
        out.push_str(&format!("export type {{ {name} }} from './{name}';\n"));
    }
    std::fs::write(out_dir.join("index.ts"), out)?;
    Ok(())
}
