# xforme

**A blazing-fast Rust engine that streams your data through ordinary Excel
templates to mass-produce pixel-perfect spreadsheets and PDFs, formatting and
formulas intact.**

xforme is the Rust migration of [*templateIt*](https://templateit.sourceforge.net/)
— the 2009 Java project by the same author — and its canonical
[Sales Receipt](https://templateit.sourceforge.net/SalesReceipt.html) example.

The idea: you describe a document once — as an ordinary Excel workbook or a
declarative Rust template — then stream a data file through it to render
**both a spreadsheet and a PDF**. The original templateIt used Apache POI for
Excel output and iText for PDF.

This port offers **two modes**:

1. **Declarative mode** — the template is defined in Rust code. Self-contained,
   no external files, PDF drawn directly with `printpdf`.
2. **Excel-template mode** — the faithful templateIt workflow: the template is a
   real **`.xlsx` workbook designed in Excel** (styles, number formats, merges
   and *formulas*). The engine fills a new tab with rendered data and removes the
   template tab, exactly like the original.

| Original (Java, 2009) | This port (Rust) |
| --------------------- | ---------------- |
| Apache POI (read/write Excel) | [`umya-spreadsheet`](https://crates.io/crates/umya-spreadsheet) (Excel-template mode) + [`rust_xlsxwriter`](https://crates.io/crates/rust_xlsxwriter) (declarative mode) |
| iText (PDF)           | LibreOffice headless convert (Excel-template mode) + [`printpdf`](https://crates.io/crates/printpdf) (declarative mode) |
| Excel template workbook | a real `.xlsx` (Excel-template mode) **or** a declarative [`Template`](src/template.rs) value (declarative mode) |

## Pipeline

```text
data file ──[data::parse]──▶ Sheet ─┐
                                     ├─[engine::process]──▶ Document ──┬─[render_xlsx]──▶ .xlsx
Template ────────────────────────────┘                                └─[render_pdf]───▶ .pdf
```

* **`data`** — parses the original tab-delimited stream format
  (`#sheet` … records … `##end`).
* **`template`** — the declarative model: output columns, the field names for
  each record label, and a *band* of output rows to emit per label.
* **`expr`** — a small arithmetic evaluator (`+ - * / ()`, identifiers) so the
  template can express line totals (`qty * price`) and a grand total
  (`subtotal + subtotal * taxrate`) without hard-coding them.
* **`engine`** — walks the records, resolves `${...}` placeholders and
  expressions against each record's fields plus running accumulators
  (e.g. `subtotal`), and produces a fully-resolved `Document`.
* **`render`** — two backends (`xlsx`, `pdf`) that consume the `Document` and
  know nothing about templates or the data format.
* **`salesreceipt`** — the bundled example template, built on the generic engine.

## Run it

**The `xforme` binary is generic** — point it at any `.xlsx` template and a data
file:

```sh
cargo run -- --template TEMPLATE.xlsx --data DATA.txt [--out PREFIX] [--no-pdf]

# e.g. with the bundled sample template + data:
cargo run -- --template templates/sales_receipt_template.xlsx --data data/sales_receipt.txt
```

It writes `PREFIX.xlsx` (template tab removed, formulas live) and, unless
`--no-pdf`, `PREFIX.pdf` via a headless LibreOffice when available. `PREFIX`
defaults to the data file's stem.

**The demo example is self-contained** — the Sales Receipt template and data are
embedded at compile time with `include_bytes!` / `include_str!`, so it needs no
external files (this is the pattern to copy for shipping a fixed report inside
your own binary, see [`examples/sales_receipt.rs`](examples/sales_receipt.rs)):

```sh
cargo run --example sales_receipt            # -> sales_receipt.xlsx
cargo run --example sales_receipt out.xlsx   # choose the output path
```

The embedded template is the committed asset
`templates/sales_receipt_template.xlsx` — edit it in Excel to change the design,
or edit `src/demo_template.rs` and regenerate it:

```sh
cargo run --bin make_sample_template  # regenerates templates/sales_receipt_template.xlsx
cargo test                            # parser, evaluator, formula-shift, file + bytes e2e
```

### Excel-template convention

The template `.xlsx` is designed by hand in Excel/LibreOffice. The engine reads
it (preserving all styling) via [`src/xlsx_template.rs`](src/xlsx_template.rs):

* **Column A of each row is a control tag** (hidden in the output):
  * `header` / `footer` — emitted once, fields from the matching data record;
  * any other tag (`row1`, `row2`, …) — a *detail* row; the contiguous run of
    detail rows is the **detail band**, emitted once per detail data record,
    matched by tag (so alternating `row1`/`row2` styles interleave in data order);
  * empty — a static row.
* **Placeholders** `${n}` inject the n-th data field; `${firstrow}`/`${lastrow}`
  give the detail band's row range (for aggregate formulas); `${row}` is the
  current row.
* **Formulas** are written natively in Excel. When a template row is replicated
  at a new output row, the engine row-shifts its relative references, so
  `=B7*D7` becomes `=B12*D12` and `=SUM(E${firstrow}:E${lastrow})` resolves to
  the actual rendered range. Absolute references (`$D$7`) are left untouched.

`templates/sales_receipt_template.xlsx` (built by the `make_sample_template`
binary) is a worked example — edit it in Excel to change the receipt's design
without touching any Rust.

## Data format

Tab-delimited, line-oriented — identical to the original example:

```text
#sheet	SalesReceipt	Sales Receipt
header	1/5/2009	22215	Jose Maria Fernandez	1010 Broadway, New York, NY 10010
row1	1	1	Introduction to Algebra	53.0
row2	2	1	Introduction to Algebra Solutions Manual	14.0
footer	.0525
##end
```

`row1`/`row2` are alternating (banded) line-item styles; `footer`'s single
field is the tax rate. The engine accumulates `qty * price` into `subtotal`,
then the footer computes tax and total from it.

## Library API

xforme ships as **both a binary and a library** (`xforme = { path = "..." }`).
The Excel-template engine accepts a template either by **file path** or as an
**in-memory byte array** (anything `Into<TemplateSource>` — `&Path`, `&[u8]`,
`&Vec<u8>`), and can write the report to a file or hand it back as bytes:

```rust
use std::path::Path;
use xforme::{data, xlsx_template};

let sheet = &data::parse(data_str)?[0];

// 1. Template from a file on disk -> write report to a file.
xlsx_template::render_to_file(Path::new("template.xlsx"), sheet, "report.xlsx")?;

// 2. Template from a byte buffer (network/DB/include_bytes!) -> report as bytes.
let template_bytes: &[u8] = /* ... */;
let report_xlsx: Vec<u8> = xlsx_template::render_to_bytes(template_bytes, sheet)?;

// 3. Or get the populated workbook to post-process before saving.
let workbook = xlsx_template::render(template_bytes, sheet)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

| Function | Template in | Report out |
| -------- | ----------- | ---------- |
| `render`         | path or bytes | `umya_spreadsheet::Workbook` |
| `render_to_file` | path or bytes | `.xlsx` file |
| `render_to_bytes`| path or bytes | `Vec<u8>` |

The declarative engine is library-only too:
`engine::process(&template, sheet) -> Document`, then
`render::render_xlsx(&doc, path)` / `render::render_pdf(&doc, path)`.

PDF for the Excel-template path is left to the caller (the binary shells out to
LibreOffice); the library returns the spreadsheet so you can convert it however
you like.

## Extending it

The declarative engine is generic — the Sales Receipt is just one `Template`. To
make a new document type, define a `Template` (columns, per-label field names,
and bands of `CellSpec`s using `Literal` / `Text` / `Expr` content) and feed it a
matching data stream. No engine changes required. For the Excel-template engine,
just design a new `.xlsx` — no Rust changes at all.
