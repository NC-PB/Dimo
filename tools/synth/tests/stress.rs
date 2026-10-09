//! T0.9: the stress document is deterministic and has the requested number of A0 sheets.

use dimo_synth::generate_stress;

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn stress_document_is_deterministic_and_has_requested_sheets() {
    let a = generate_stress(7, 3);
    let b = generate_stress(7, 3);
    assert_eq!(a, b, "same seed and sheet count must give identical bytes");
    assert_ne!(a, generate_stress(8, 3), "another seed must differ");
    assert!(a.starts_with(b"%PDF-1.4"));
    assert!(contains(&a, b"/Count 3 "));
    assert!(contains(&a, b"/MediaBox [0 0 3370 2384]"));
    // No Info dictionary, no ID, no dates (AGENTS.md rule 11).
    assert!(!contains(&a, b"/Info"));
    assert!(!contains(&a, b"/ID"));
    assert!(!contains(&a, b"/CreationDate"));
}
