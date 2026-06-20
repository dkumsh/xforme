//! The showcase demo: render a **Portfolio Statement** from a template and data
//! **embedded into the binary at compile time** via `include_bytes!` /
//! `include_str!`. No external files are needed to run it.
//!
//! ```sh
//! cargo run --example portfolio_statement              # -> portfolio_statement.xlsx
//! cargo run --example portfolio_statement out.xlsx     # choose the output path
//! ```
//!
//! The template is an ordinary `.xlsx` a designer could author by hand. It packs
//! everything the clone-and-edit engine preserves *and grows* with the data:
//! an embedded logo image, conditional formatting (red/green on the gain
//! column), in-cell data bars over the market-value column, and two column
//! charts whose series ranges expand as holdings are added. Open the output and
//! the formulas (line gain/loss, the totals) are live.

use xforme::{data, xlsx_template};

/// The Excel template, compiled into this example's binary.
static TEMPLATE: &[u8] = include_bytes!("../templates/portfolio_statement_template.xlsx");

/// The data stream, compiled in as well.
const DATA: &str = include_str!("../data/portfolio_statement.json");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/portfolio_statement.xlsx".to_string());

    // Parse the embedded data and render it through the embedded template.
    let sheets = data::parse_json(DATA)?;
    let sheet = sheets.first().ok_or("embedded data contained no sheets")?;

    // `render_to_file` accepts the template as bytes (here) or a path.
    let warnings = xlsx_template::render_to_file(TEMPLATE, sheet, &out_path)?;
    for w in &warnings {
        eprintln!("warning: {w}");
    }
    println!("Wrote {out_path} from the embedded template + data");

    Ok(())
}
