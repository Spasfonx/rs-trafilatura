use rs_trafilatura::extract;

const PADDING: &str = "<p>Additional paragraph to ensure sufficient content for the extraction algorithm to consider this a real article.</p><p>Second padding paragraph with more text to satisfy the minimum content scoring threshold for table extraction.</p>";

#[test]
fn extract_formats_simple_tables_in_content_text_and_preserves_in_content_html() {
    let html = format!(r#"
        <article>
            <p>Intro text for the article with enough content.</p>
            {PADDING}
            <table>
                <tr><th>H1</th><th>H2</th></tr>
                <tr><td>A</td><td>B</td></tr>
            </table>
        </article>
    "#);

    let result = extract(&html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("H1 | H2"));
            assert!(result.content_text.contains("A | B"));

            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("<table>"));
            assert!(content_html.contains("<tr>"));
            assert!(content_html.contains("<th>H1</th>"));
            assert!(content_html.contains("<td>A</td>"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_treats_layout_tables_as_regular_content() {
    let html = r#"
        <article>
            <table role="presentation">
                <tr><td><p>LAYOUT_MARKER</p></td></tr>
            </table>
        </article>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("LAYOUT_MARKER"));
            assert!(!result.content_text.contains('|'));

            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(!content_html.contains("<table"));
            assert!(content_html.contains("<p>LAYOUT_MARKER</p>"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_handles_colspan_and_rowspan_in_table_text() {
    let html = format!(r#"
        <article>
            {PADDING}
            <table>
                <tr><th>H1</th><th>H2</th></tr>
                <tr><td colspan="2">X</td></tr>
                <tr><td rowspan="2">A</td><td>B1</td></tr>
                <tr><td>B2</td></tr>
            </table>
        </article>
    "#);

    let result = extract(&html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("X | X"));
            assert!(result.content_text.contains("A | B1"));
            assert!(result.content_text.contains("A | B2"));

            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains(r#"<td colspan="2">X</td>"#));
            assert!(content_html.contains(r#"<td rowspan="2">A</td>"#));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_handles_large_tables_without_panic() {
    let mut table = format!("<article>{PADDING}<table>");
    table.push_str("<tr><th>H1</th><th>H2</th><th>H3</th></tr>");
    for r in 0..200 {
        table.push_str("<tr>");
        for c in 0..20 {
            table.push_str(&format!("<td>R{r}C{c}</td>"));
        }
        table.push_str("</tr>");
    }
    table.push_str("</table></article>");

    let result = extract(&table);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("H1 | H2 | H3"));
            assert!(result.content_text.contains("R0C0"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_treats_single_row_table_as_layout() {
    let html = format!(r#"
        <article>
            {PADDING}
            <table>
                <tr><td>SINGLE_ROW_CONTENT</td><td>More</td></tr>
            </table>
            <p>BODY_TEXT</p>
        </article>
    "#);

    let result = extract(&html);
    match result {
        Ok(result) => {
            // Single row table is treated as layout - content extracted but no pipe separators
            assert!(result.content_text.contains("SINGLE_ROW_CONTENT"));
            assert!(result.content_text.contains("BODY_TEXT"));
            // Layout tables don't get pipe-formatted
            assert!(!result.content_text.contains("SINGLE_ROW_CONTENT | More"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

// `colspan` / `rowspan` are page-controlled numbers. Before they were clamped,
// `extract_table_text` sized its rowspan bookkeeping vector from the raw
// `colspan`, so a single `<td colspan="2000000000">` requested 64 GB and the
// process died (SIGKILL under a cgroup limit, `memory allocation failed` abort
// otherwise). Clamps follow the HTML spec: colspan ≤ 1000, rowspan ≤ 65534.
#[test]
fn extract_survives_absurd_colspan() {
    let html = format!(
        r#"
        <article>
            <p>Intro text for the article with enough content.</p>
            {PADDING}
            <table>
                <tr><th>H1</th><th>H2</th></tr>
                <tr><td colspan="2000000000">WIDE</td><td>B</td></tr>
                <tr><td>1</td><td>2</td></tr>
            </table>
        </article>
    "#
    );

    let result = extract(&html).expect("expected Ok(_)");
    assert!(result.content_text.contains("H1 | H2"));
    assert!(result.content_text.contains("WIDE"));
    assert!(result.content_text.contains("1 | 2"));
}

#[test]
fn extract_survives_absurd_rowspan() {
    let html = format!(
        r#"
        <article>
            <p>Intro text for the article with enough content.</p>
            {PADDING}
            <table>
                <tr><th>H1</th><th>H2</th></tr>
                <tr><td rowspan="4000000000">TALL</td><td>B</td></tr>
                <tr><td>2</td></tr>
            </table>
        </article>
    "#
    );

    let result = extract(&html).expect("expected Ok(_)");
    assert!(result.content_text.contains("H1 | H2"));
    assert!(result.content_text.contains("TALL | B"));
    assert!(result.content_text.contains("TALL | 2"));
}
