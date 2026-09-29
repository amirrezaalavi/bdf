//! TDD: the reason code set is a public contract — its serialized names are stable.

#[test]
fn reasons_serialize_to_stable_snake_case() {
    let json = serde_json::to_string(&pdfrtl_core::Reason::UnsupportedNoEvidence).unwrap();
    assert_eq!(json, "\"unsupported_no_evidence\"");
}

#[test]
fn every_reason_has_a_stable_string_form() {
    use pdfrtl_core::Reason::*;
    let all = [
        (ActualText, "actual_text"),
        (ToUnicodeLogical, "to_unicode_logical"),
        (BidiReordered, "bidi_reordered"),
        (ProducerVisualOrderKnown, "producer_visual_order_known"),
        (EncodingMapped, "encoding_mapped"),
        (UnsupportedNoEvidence, "unsupported_no_evidence"),
        (UnsupportedBrokenToUnicode, "unsupported_broken_to_unicode"),
        (UnsupportedFontEncoding, "unsupported_font_encoding"),
        (UnsupportedPageContent, "unsupported_page_content"),
        (UnsupportedVisualOrder, "unsupported_visual_order"),
    ];
    for (reason, expected) in all {
        assert_eq!(reason.as_str(), expected);
        assert_eq!(
            serde_json::to_string(&reason).unwrap(),
            format!("\"{expected}\"")
        );
    }
}

#[test]
fn unsupported_reasons_are_the_only_ones_flagged_unsupported() {
    use pdfrtl_core::Reason::*;
    assert!(UnsupportedNoEvidence.is_unsupported());
    assert!(UnsupportedBrokenToUnicode.is_unsupported());
    assert!(UnsupportedFontEncoding.is_unsupported());
    assert!(UnsupportedPageContent.is_unsupported());
    assert!(UnsupportedVisualOrder.is_unsupported());
    assert!(!ActualText.is_unsupported());
    assert!(!BidiReordered.is_unsupported());
    assert!(!ProducerVisualOrderKnown.is_unsupported());
    assert!(!ToUnicodeLogical.is_unsupported());
    assert!(!EncodingMapped.is_unsupported());
}
