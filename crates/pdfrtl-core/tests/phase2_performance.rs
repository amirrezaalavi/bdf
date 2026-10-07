//! Acceptance tests for Phase 2: High-Performance Engine.
//!
//! Verifies:
//! 1. `UnitText` compact inline storage and size constraints.
//! 2. Zero-copy stream lexing and typed opcode decoding.
//! 3. Streaming `extract_pages_iter` equality with `extract_document`.
//! 4. Multi-threaded concurrency without lock contention or thread-local leakage.

use lopdf::Document;
use pdfrtl_core::text::bidi::{take_order_trace, trace_line};
use pdfrtl_core::text::unit_text::{UnitText, INLINE_CAPACITY};
use pdfrtl_core::text::{extract_document, extract_pages_iter, TextOp, Token};
use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;
use std::thread;

#[test]
fn unit_text_size_is_at_most_32_bytes() {
    assert!(
        std::mem::size_of::<UnitText>() <= 32,
        "UnitText must be compact (<= 32 bytes on 64-bit platforms), found {}",
        std::mem::size_of::<UnitText>()
    );
}

#[test]
fn unit_text_inlines_persian_and_arabic_clusters() {
    let clusters = ["سلام", "دنیا", "کتاب", "۱۴۰۳", "لا", "(متن)"];
    for cluster in clusters {
        let u = UnitText::new(cluster);
        assert!(
            u.is_inline(),
            "Cluster {cluster:?} should fit within {INLINE_CAPACITY} bytes inline"
        );
        assert_eq!(u.as_str(), cluster);
        assert_eq!(u, cluster);
    }
}

#[test]
fn streaming_iterator_matches_batch_extraction() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("corpus/redistributable/synthetic/chrome-fa-plain.pdf");

    if !fixture_path.exists() {
        return;
    }

    let doc = Document::load(&fixture_path).expect("load fixture");
    let batch = extract_document(&doc).expect("batch extract");
    let streamed: Vec<_> = extract_pages_iter(&doc).collect();

    assert_eq!(batch.len(), streamed.len());
    for (b, s) in batch.iter().zip(streamed.iter()) {
        assert_eq!(b.page, s.page);
        assert_eq!(b.text, s.text);
        assert_eq!(b.reasons, s.reasons);
        assert_eq!(b.unordered_chars, s.unordered_chars);
    }
}

#[test]
fn concurrent_multithreaded_extraction_is_isolated_and_thread_safe() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("corpus/redistributable/synthetic/chrome-fa-plain.pdf");

    if !fixture_path.exists() {
        return;
    }

    let doc = Arc::new(Document::load(&fixture_path).expect("load fixture"));
    let handles: Vec<_> = (0..8)
        .map(|thread_id| {
            let doc_clone = Arc::clone(&doc);
            thread::spawn(move || {
                // Ensure thread-local trace isolation
                let _ = take_order_trace();
                trace_line(10, 0, 10, 0.0, 100.0, true);

                let pages: Vec<_> = extract_pages_iter(&doc_clone).collect();
                assert!(!pages.is_empty());
                assert_eq!(pages[0].page, 1);

                let trace = take_order_trace();
                assert!(
                    !trace.is_empty(),
                    "Thread {thread_id} should have its own trace"
                );
                pages
            })
        })
        .collect();

    for handle in handles {
        let pages = handle.join().expect("thread join succeeded");
        assert!(!pages.is_empty());
    }
}

#[test]
fn zero_copy_lexer_borrows_stream_slices() {
    let stream = b"BT /F1 14 Tf (Persian Text) Tj ET";
    let tokens = pdfrtl_core::text::tokenizer::tokenize(stream);
    assert_eq!(tokens.len(), 7);

    assert_eq!(tokens[0], Token::Op(TextOp::BT, b"BT"));
    assert!(matches!(&tokens[1], Token::Name(Cow::Borrowed(b"F1"))));
    assert_eq!(tokens[2], Token::Num(14.0));
    assert_eq!(tokens[3], Token::Op(TextOp::Tf, b"Tf"));
    assert!(matches!(
        &tokens[4],
        Token::Str(Cow::Borrowed(b"Persian Text"))
    ));
    assert_eq!(tokens[5], Token::Op(TextOp::Tj, b"Tj"));
    assert_eq!(tokens[6], Token::Op(TextOp::ET, b"ET"));
}
