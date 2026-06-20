//! `xforme` — a generic command-line renderer.
//!
//! Streams a tab-delimited data file through an Excel `.xlsx` template and
//! writes the rendered report: the template tab is removed from the result and
//! all formatting and formulas are preserved.
//!
//! ```text
//! xforme --template TEMPLATE.xlsx --data DATA.txt [--out PREFIX] [--no-pdf]
//! ```
//!
//! Produces `PREFIX.xlsx` and, unless `--no-pdf`, `PREFIX.pdf` via a headless
//! LibreOffice when one is available. `PREFIX` defaults to the data file's stem.
//!
//! For a self-contained example with a template and data embedded at compile
//! time, see `cargo run --example sales_receipt`.

use std::path::Path;
use std::process::ExitCode;

use xforme::{data, xlsx_template};

struct Args {
    template: String,
    data: String,
    out: Option<String>,
    pdf: bool,
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(msg) => {
            if msg != "help" {
                eprintln!("error: {msg}");
            }
            eprintln!(
                "usage: xforme --template TEMPLATE.xlsx --data DATA.txt [--out PREFIX] [--no-pdf]"
            );
            return if msg == "help" {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
    };

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn parse_args() -> Result<Args, String> {
    let mut template = None;
    let mut data = None;
    let mut out = None;
    let mut pdf = true;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--template" => template = Some(it.next().ok_or("--template requires a path")?),
            "--data" => data = Some(it.next().ok_or("--data requires a path")?),
            "--out" => out = Some(it.next().ok_or("--out requires a prefix")?),
            "--no-pdf" => pdf = false,
            "-h" | "--help" => return Err("help".into()),
            other => return Err(format!("unexpected argument `{other}`")),
        }
    }

    Ok(Args {
        template: template.ok_or("missing required --template TEMPLATE.xlsx")?,
        data: data.ok_or("missing required --data DATA.txt")?,
        out,
        pdf,
    })
}

fn run(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let raw =
        std::fs::read_to_string(&args.data).map_err(|e| format!("reading {}: {e}", args.data))?;
    let sheets = parse_data(&args.data, &raw)?;
    let sheet = sheets.first().ok_or("data file contained no sheets")?;

    let prefix = args
        .out
        .clone()
        .unwrap_or_else(|| default_prefix(&args.data));
    let xlsx_path = format!("{prefix}.xlsx");

    let warnings = xlsx_template::render_to_file(Path::new(&args.template), sheet, &xlsx_path)?;
    for w in &warnings {
        eprintln!("warning: {w}");
    }
    println!("Wrote {xlsx_path} (template tab removed, formulas live)");

    if args.pdf {
        emit_pdf(&xlsx_path);
    }
    Ok(())
}

/// Parse the data file, choosing the format by extension: `.json` and
/// `.yaml`/`.yml` use the serde parsers (when their features are enabled);
/// everything else is the tab-delimited format.
// The `return`s below are the tail of cfg-gated arms; one variant is always
// compiled out, which makes the survivor look "needless" to clippy.
#[allow(clippy::needless_return)]
fn parse_data(path: &str, raw: &str) -> Result<Vec<data::Sheet>, Box<dyn std::error::Error>> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "json" => {
            #[cfg(feature = "json")]
            {
                return data::parse_json(raw);
            }
            #[cfg(not(feature = "json"))]
            return Err("built without the `json` feature".into());
        }
        "yaml" | "yml" => {
            #[cfg(feature = "yaml")]
            {
                return data::parse_yaml(raw);
            }
            #[cfg(not(feature = "yaml"))]
            return Err("built without the `yaml` feature".into());
        }
        "csv" => {
            #[cfg(feature = "csv")]
            {
                return data::parse_csv(raw);
            }
            #[cfg(not(feature = "csv"))]
            return Err("built without the `csv` feature".into());
        }
        _ => Ok(data::parse(raw)?),
    }
}

/// Default output prefix: the data file's stem, or `output`.
fn default_prefix(data_path: &str) -> String {
    Path::new(data_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string()
}

/// Convert the rendered `.xlsx` to PDF via LibreOffice (the `pdf` feature).
#[cfg(feature = "pdf")]
fn emit_pdf(xlsx_path: &str) {
    match xforme::pdf::to_pdf_file(xlsx_path) {
        Ok(pdf) => println!("Wrote {}", pdf.display()),
        Err(e) => eprintln!(
            "note: skipped PDF ({e}). Install LibreOffice, or open {xlsx_path} and export to PDF."
        ),
    }
}

/// Without the `pdf` feature the binary emits `.xlsx` only.
#[cfg(not(feature = "pdf"))]
fn emit_pdf(xlsx_path: &str) {
    eprintln!(
        "note: built without the `pdf` feature; wrote {xlsx_path} only — convert it yourself."
    );
}
