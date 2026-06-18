//! Optional PDF output (Cargo feature `pdf`, on by default).
//!
//! `.xlsx` is xforme's product; PDF is produced by converting a rendered
//! workbook with a **headless LibreOffice** — the only way to get an
//! Excel-faithful PDF (formulas evaluated, formatting/pagination applied)
//! without reimplementing Excel's engine.
//!
//! Because this spawns `soffice`/`libreoffice` as a subprocess, it lives behind
//! the `pdf` feature: building with `default-features = false` yields a crate
//! that can't execute a subprocess at all.
//!
//! LibreOffice converts *files*, not memory, so [`to_pdf_bytes`] round-trips
//! through a temporary directory internally. Note: conversions share one
//! LibreOffice user profile, so heavily concurrent calls may contend — convert
//! sequentially if that matters.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Whether a LibreOffice binary (`soffice` or `libreoffice`) is on `PATH`.
pub fn libreoffice_available() -> bool {
    which_soffice().is_some()
}

/// Convert an existing `.xlsx` file to a PDF in the same directory, returning
/// the PDF path. Errors if LibreOffice is not found.
pub fn to_pdf_file(xlsx_path: impl AsRef<Path>) -> Result<PathBuf> {
    let xlsx_path = xlsx_path.as_ref();
    let out_dir = xlsx_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    convert(xlsx_path, out_dir)?;
    let pdf = xlsx_path.with_extension("pdf");
    if !pdf.exists() {
        return Err("LibreOffice reported success but produced no PDF".into());
    }
    Ok(pdf)
}

/// Convert `.xlsx` bytes to PDF bytes, round-tripping through a temp directory
/// (LibreOffice can't read our in-memory workbook directly).
pub fn to_pdf_bytes(xlsx: &[u8]) -> Result<Vec<u8>> {
    let dir = unique_temp_dir()?;
    let xlsx_path = dir.join("xforme.xlsx");
    let pdf_path = dir.join("xforme.pdf");
    let result = (|| {
        std::fs::write(&xlsx_path, xlsx)?;
        convert(&xlsx_path, &dir)?;
        Ok(std::fs::read(&pdf_path)?)
    })();
    let _ = std::fs::remove_dir_all(&dir); // best-effort cleanup
    result
}

fn convert(xlsx_path: &Path, out_dir: &Path) -> Result<()> {
    let soffice = which_soffice().ok_or("LibreOffice (soffice/libreoffice) not found on PATH")?;
    let output = std::process::Command::new(soffice)
        .arg("--headless")
        // Isolated profile so a running desktop instance doesn't lock us out.
        .arg("-env:UserInstallation=file:///tmp/xforme_lo_profile")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(out_dir)
        .arg(xlsx_path)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "soffice failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

fn which_soffice() -> Option<&'static str> {
    ["soffice", "libreoffice"].into_iter().find(|c| {
        std::process::Command::new(c)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

fn unique_temp_dir() -> Result<PathBuf> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("xforme-{}-{}", std::process::id(), n));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
