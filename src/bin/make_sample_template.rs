//! Exports the bundled sample templates to `templates/*.xlsx` so you can open
//! them in Excel/LibreOffice and redesign them by hand.
//!
//! The examples embed these same templates at compile time with `include_bytes!`,
//! so this is only needed when you want editable copies on disk.
//!
//! Run with: `cargo run --bin make_sample_template`

fn main() {
    std::fs::create_dir_all("templates").expect("create templates dir");
    for (path, bytes) in [
        (
            "templates/sales_receipt_template.xlsx",
            xforme::demo_template::sample_sales_receipt_template_bytes(),
        ),
        (
            "templates/portfolio_statement_template.xlsx",
            xforme::demo_template::sample_portfolio_statement_template_bytes(),
        ),
    ] {
        std::fs::write(path, bytes).expect("write template");
        println!("Wrote {path}");
    }
}
