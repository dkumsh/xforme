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
    let sheets = data::parse(&raw)?;
    let sheet = sheets.first().ok_or("data file contained no sheets")?;

    let prefix = args
        .out
        .clone()
        .unwrap_or_else(|| default_prefix(&args.data));
    let xlsx_path = format!("{prefix}.xlsx");

    xlsx_template::render_to_file(Path::new(&args.template), sheet, &xlsx_path)?;
    println!("Wrote {xlsx_path} (template tab removed, formulas live)");

    if args.pdf {
        match convert_to_pdf(&xlsx_path) {
            Ok(pdf_path) => println!("Wrote {pdf_path}"),
            Err(e) => eprintln!(
                "note: skipped PDF ({e}). Install LibreOffice, or open {xlsx_path} and export to PDF."
            ),
        }
    }
    Ok(())
}

/// Default output prefix: the data file's stem, or `output`.
fn default_prefix(data_path: &str) -> String {
    Path::new(data_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string()
}

/// Convert a spreadsheet to PDF using a headless LibreOffice, the modern
/// equivalent of the original's iText spreadsheet->PDF step. LibreOffice
/// recalculates the live formulas during conversion.
fn convert_to_pdf(xlsx_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let soffice = which_soffice().ok_or("LibreOffice (soffice) not found on PATH")?;

    let path = Path::new(xlsx_path);
    let out_dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    let output = std::process::Command::new(soffice)
        .arg("--headless")
        // Use an isolated profile so a running desktop instance doesn't lock us out.
        .arg("-env:UserInstallation=file:///tmp/xforme_lo_profile")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(out_dir)
        .arg(path)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "soffice failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let pdf_path = path.with_extension("pdf");
    if !pdf_path.exists() {
        return Err("soffice reported success but no PDF was produced".into());
    }
    Ok(pdf_path.to_string_lossy().into_owned())
}

fn which_soffice() -> Option<String> {
    for candidate in ["soffice", "libreoffice"] {
        if std::process::Command::new(candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(candidate.to_string());
        }
    }
    None
}
