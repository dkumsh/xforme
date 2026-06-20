# AGENTS.md — working on xforme

xforme streams a data file through an ordinary `.xlsx` **template** and emits a
populated `.xlsx` (optionally a PDF), preserving styles, formulas, conditional
formatting, data bars, images, and charts.

```text
data file ──[data::parse*]──▶ Sheet ──┐
                                      ├─[xlsx_template::render*]──▶ .xlsx report
template.xlsx ────────────────────────┘
```

## Where to look

- **Full guide (template authoring + API, for any agent): [`.claude/skills/xforme/SKILL.md`](.claude/skills/xforme/SKILL.md)** — read it before creating/editing a template or wiring the API. (It's a Claude Code skill, but it's a plain Markdown doc — open and read it directly.)
- End-user docs and showcase: [`README.md`](README.md).
- Canonical worked example to copy: [`src/demo_template.rs`](src/demo_template.rs).
- Engine + contract: [`src/xlsx_template.rs`](src/xlsx_template.rs); data formats: [`src/data.rs`](src/data.rs).

## Build / verify

```sh
just check    # rustfmt + clippy
just test     # parser, parameter resolution, formula-shift, e2e
```

Default features are `pdf json yaml csv`; also verify `cargo build --no-default-features`.
PDF and screenshots need LibreOffice (`soffice`) on `PATH`.

## Critical traps when generating a template with `umya-spreadsheet` 3.0

These break output **silently** and are not in umya's docs. The full list (with
code) is in the skill; the ones that bite hardest:

1. **No page setup on a template with images/charts.** umya mis-numbers the
   drawing relationship id when page setup has any parameter, and Excel/
   LibreOffice then drop the *entire* drawing layer. Don't call
   `set_orientation` / `set_fit_to_width` / etc. on such sheets.
2. **Always set an explicit font color on styled cells.** A bold `Style` with no
   color serializes as `indexed="1"` (white) → invisible text.
3. **`cellIs` CF rules need `set_formula(Formula::set_string_value("0"))`**, not
   `set_text`.
4. **Always verify visually** — render to `.xlsx`, convert with LibreOffice, and
   eyeball the PNG. XML inspection alone misses dropped drawings and invisible
   text.

## Conventions

- Match the surrounding code's style; keep `just check` green before committing.
- Single-line commit messages (see `git log`).
- `0.2.0` is **published to crates.io**; the `v0.2.0` tag is frozen at that
  commit — do **not** move it. New crate-facing changes go under `[Unreleased]`
  in `CHANGELOG.md` and warrant a version bump at release.
