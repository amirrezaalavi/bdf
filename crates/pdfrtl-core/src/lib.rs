//! pdfrtl core: document model, RTL text pipeline, extraction, generation.
//!
//! Invariant (do not weaken): every text result is either in logical order or
//! carries an explicit [`Reason`] explaining why we refused to produce one.
//! Silent reversal is a bug, never a fallback.
pub mod docinfo;
pub mod reasons;

pub use docinfo::{inspect, DocInfo};
pub use reasons::Reason;
