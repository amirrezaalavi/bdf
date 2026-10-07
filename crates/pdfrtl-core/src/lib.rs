//! pdfrtl core: document model, RTL text pipeline, extraction, generation.
//!
//! Invariant (do not weaken): every text result is either in logical order or
//! carries an explicit [`Reason`] explaining why we refused to produce one.
//! Silent reversal is a bug, never a fallback.
pub mod docinfo;
pub mod layout;
pub mod reasons;
pub mod search;
pub mod text;

pub use docinfo::{inspect, DocInfo};
pub use layout::{PageLayout, TextBlock, TextLine, TextSpan};
pub use reasons::Reason;
pub use text::{extract, extract_document, PageText};

/// Recover spatial layout with bounding boxes for all pages of a PDF file.
pub fn extract_layout(path: &std::path::Path) -> anyhow::Result<Vec<PageLayout>> {
    let doc = lopdf::Document::load(path)?;
    extract_layout_document(&doc)
}

/// Recover spatial layout from an already-loaded document.
pub fn extract_layout_document(doc: &lopdf::Document) -> anyhow::Result<Vec<PageLayout>> {
    let pages = extract_document(doc)?;
    Ok(pages
        .into_iter()
        .map(|p| {
            let bbox = PageLayout::compute_bbox(&p.blocks);
            PageLayout {
                page: p.page,
                bbox,
                blocks: p.blocks,
            }
        })
        .collect())
}
