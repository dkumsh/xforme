//! Output backends that turn a [`Document`](crate::document::Document) into a
//! file. Each renderer is independent of the engine and template machinery.

pub mod pdf;
pub mod xlsx;

pub use pdf::render_pdf;
pub use xlsx::render_xlsx;
