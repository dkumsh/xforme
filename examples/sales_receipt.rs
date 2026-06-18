//! Self-contained demo: render a Sales Receipt with the template and data
//! **embedded into the binary at compile time** via `include_bytes!` /
//! `include_str!`. No external files are needed to run it.
//!
//! ```sh
//! cargo run --example sales_receipt            # -> sales_receipt.xlsx
//! cargo run --example sales_receipt out.xlsx   # choose the output path
//! ```
//!
//! This is the pattern to copy when you want to ship a fixed report inside your
//! own binary: bake the `.xlsx` template in with `include_bytes!`, then feed it
//! to [`xforme::xlsx_template`] alongside your data.

use xforme::{data, xlsx_template};

/// The Excel template, compiled into this example's binary.
static TEMPLATE: &[u8] = include_bytes!("../templates/sales_receipt_template.xlsx");

/// The data stream, compiled in as well.
const DATA: &str = include_str!("../data/sales_receipt.txt");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/sales_receipt.xlsx".to_string());

    // Parse the embedded data and render it through the embedded template.
    let sheets = data::parse(DATA)?;
    let sheet = sheets.first().ok_or("embedded data contained no sheets")?;

    // `render_to_file` accepts the template as bytes (here) or a path.
    xlsx_template::render_to_file(TEMPLATE, sheet, &out_path)?;
    println!("Wrote {out_path} from the embedded template + data");

    Ok(())
}
