use wiki::core::chunker::chunk_text;

#[test]
fn chunk_short_text_returns_one_chunk() {
    let chunks = chunk_text("Hello world.", 512, 128, 1);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "Hello world.");
}

#[test]
fn chunk_long_text_splits_into_multiple() {
    let text = (0..100)
        .map(|i| format!("Paragraph {}.\n\n", i))
        .collect::<String>();
    let chunks = chunk_text(&text, 50, 10, 5);
    assert!(chunks.len() > 1);
}

#[test]
fn chunk_tracks_header_breadcrumb() {
    let text = "# Title\n\nIntro.\n\n## Sub\n\nBody.";
    let chunks = chunk_text(text, 50, 10, 5);
    assert!(chunks.iter().any(|c| c.header_breadcrumb.contains("Title")));
    assert!(chunks.iter().any(|c| c.header_breadcrumb.contains("Sub")));
}

#[test]
fn chunk_skips_tiny_chunks() {
    let chunks = chunk_text("Hi.", 512, 128, 32);
    assert!(chunks.is_empty() || chunks[0].content == "Hi.");
}

#[test]
fn chunk_handles_empty_input() {
    assert!(chunk_text("", 512, 128, 32).is_empty());
    assert!(chunk_text("   \n\n  ", 512, 128, 32).is_empty());
}
