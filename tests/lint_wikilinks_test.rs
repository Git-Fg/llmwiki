use wiki::lint::wikilinks::check_wikilinks;

#[test]
fn dangling_link_is_error() {
    let body = "See [[nonexistent]].";
    let issues = check_wikilinks("wiki/a.md", body, &[], 0, 10);
    assert!(issues.iter().any(|i| i.code == "dangling-link"));
}

#[test]
fn valid_link_passes() {
    let body = "See [[existing]].";
    let pages = vec!["wiki/existing.md".to_string()];
    let issues = check_wikilinks("wiki/a.md", body, &pages, 1, 10);
    assert!(issues.iter().all(|i| i.code != "dangling-link"));
}

#[test]
fn zero_outbound_is_error() {
    let body = "No links here.";
    let issues = check_wikilinks("wiki/a.md", body, &[], 1, 10);
    assert!(issues
        .iter()
        .any(|i| i.code == "no-outbound-links" && i.severity == "error"));
}

#[test]
fn one_outbound_is_error_needs_two() {
    let body = "Only [[one-link]].";
    let pages = vec!["wiki/one-link.md".to_string()];
    let issues = check_wikilinks("wiki/a.md", body, &pages, 1, 10);
    assert!(issues
        .iter()
        .any(|i| i.code == "below-min-outbound" && i.severity == "error"));
}

#[test]
fn two_outbound_passes() {
    let body = "Links: [[a]] and [[b]].";
    let pages = vec!["wiki/a.md".to_string(), "wiki/b.md".to_string()];
    let issues = check_wikilinks("wiki/a.md", body, &pages, 1, 10);
    assert!(issues.is_empty(), "{:?}", issues);
}

#[test]
fn orphan_page_is_error() {
    let body = "Links: [[a]] and [[b]].";
    let pages = vec!["wiki/a.md".to_string(), "wiki/b.md".to_string()];
    let issues = check_wikilinks("wiki/isolated.md", body, &pages, 0, 10);
    assert!(issues.iter().any(|i| i.code == "orphan-page"));
}

#[test]
fn large_page_is_warn() {
    let body = "x\n".repeat(250);
    let issues = check_wikilinks("wiki/big.md", &body, &[], 1, 250);
    assert!(issues.iter().any(|i| i.code == "page-too-long"));
}
