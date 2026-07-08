//! Regression tests for the Python-trafilatura baseline-rescue alignment: the
//! extractor used to return empty `content_text` on `<form>`-wrapped pages
//! (classic ASP.NET WebForms — the whole body inside one `<form runat="server">`).
//!
//! `<form>` is in `TAGS_TO_CLEAN`, so `doc_cleaning` removes the form *and all
//! its children*. When the entire body is form-wrapped, the main extraction is
//! therefore empty and only the `baseline` rescue — which runs on the
//! pre-cleaning backup — can recover the article. Two divergences from Python
//! trafilatura 2.1.0 left that rescue empty (both fixed in this change):
//!
//!   1. `basic_cleaning` was far broader than Python's `BASIC_CLEAN_XPATH`: it
//!      stripped `nav`/`header`/`modal`/`banner`/`popup`/`consent`/… containers,
//!      so an article nested in such a container was deleted by the rescue too.
//!   2. `baseline` returned partial paragraphs even when their text was <= 100
//!      chars, so a short `<p>` title pre-empted the whole-`<body>` scrape and
//!      hid text carried in tables / non-`<p>` nodes.
//!
//! Each case below reproduces one divergence with a minimal, fully synthetic
//! page (no scraped content) and asserts recovery under the crawler's production
//! options. On the pre-fix code every case returns empty `content_text`.

use rs_trafilatura::{extract_with_options, Options};

/// Precision focus with the fallback rescue enabled — mirrors the production
/// (crawler) configuration under which these pages returned empty.
fn crawler_options() -> Options {
    Options {
        favor_precision: true,
        use_fallback_extraction: true,
        include_formatting: true,
        include_comments: false,
        ..Options::default()
    }
}

const ARTICLE: &str = "The regional council approved the new transit plan on Tuesday, \
    ending a two-year debate over how to connect the eastern suburbs to the city centre \
    without widening the existing motorway that residents rely on every day.";

/// Divergence 1: the article is nested in a container whose class matched the
/// old over-broad `basic_cleaning` selector (`banner`), inside a single ASP.NET
/// `<form>`. The narrowed, Python-faithful selector must keep it.
#[test]
fn aspnet_form_with_boilerplate_class_container_is_recovered() {
    let html = format!(
        r#"<!DOCTYPE html><html><head><title>News</title></head><body>
        <form id="form1" name="form1" method="post" runat="server">
        <input type="hidden" name="__VIEWSTATE" value="/wEPDwUJLTE5MjIxNTU5ZA==" />
        <div class="banner-inner">
            <p>{ARTICLE}</p>
            <p>{ARTICLE}</p>
        </div>
        </form></body></html>"#
    );

    let result = extract_with_options(&html, &crawler_options()).expect("extraction should be Ok");

    assert!(
        result.content_text.contains("regional council approved"),
        "a form-wrapped article in a banner-class container must be recovered, got: {:?}",
        result.content_text
    );
}

/// Divergence 2: a `<form>`-wrapped page whose body text lives outside `<p>`
/// (in a table), behind a short `<p>` title (<= 100 chars). `baseline` must fall
/// through to the whole-`<body>` scrape rather than returning only the title.
#[test]
fn aspnet_form_with_table_body_is_recovered() {
    let html = format!(
        r#"<!DOCTYPE html><html><head><title>Episode Guide</title></head><body>
        <form id="form1" name="form1" method="post" runat="server">
        <input type="hidden" name="__VIEWSTATE" value="/wEPDwUJLTE5MjIxNTU5ZA==" />
        <p>Overview.</p>
        <table>
            <tr><td>Summary</td><td>{ARTICLE}</td></tr>
            <tr><td>Detail</td><td>{ARTICLE}</td></tr>
        </table>
        </form></body></html>"#
    );

    let result = extract_with_options(&html, &crawler_options()).expect("extraction should be Ok");

    assert!(
        result.content_text.contains("regional council approved"),
        "form-wrapped text carried in a table must be recovered, got: {:?}",
        result.content_text
    );
}
