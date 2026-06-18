//! Exports the bundled sample template to `templates/sales_receipt_template.xlsx`
//! so you can open it in Excel/LibreOffice and redesign the receipt by hand.
//!
//! The demo binary embeds this same template at compile time (see `build.rs`),
//! so this is only needed when you want an editable copy on disk.
//!
//! Run with: `cargo run --bin make_sample_template`

fn main() {
    std::fs::create_dir_all("templates").expect("create templates dir");
    let path = "templates/sales_receipt_template.xlsx";
    std::fs::write(
        path,
        xforme::demo_template::sample_sales_receipt_template_bytes(),
    )
    .expect("write template");
    println!("Wrote {path}");
}
