//! The page's Content-Security-Policy, written into its built `index.html` after Trunk. It names the hashes of the
//! loader script and the styles Trunk writes inline, so the browser runs no other script and applies no other style,
//! and it lets the page fetch only from its own address: the picture dropped in never leaves the browser, and the
//! browser enforces it.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};

/// Why a built page gets no policy: a policy written for it could protect it in name only, or break it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No `<meta charset="utf-8" />` to write the policy right after.
    NoCharset,
    /// Line ends with carriage returns, which a browser drops before hashing and the hook would not.
    CarriageReturns,
    /// A comment, which could hide a charset, a script or a style the hook would read as real.
    Comments,
    /// A script or style tag in capitals, which the hook would not hash.
    TagsInCapitals,
    /// A script or style tag whose attribute holds a `>`, which the hook would take for the tag's end.
    AnglesInATag,
    /// A script or style tag never closed, whose body the hook cannot tell.
    UnclosedTag,
}

const CHARSET: &str = r#"<meta charset="utf-8" />"#;

/// The page with its policy right after its charset, hashing every inline script and style it holds, or refused when
/// such a policy could protect it in name only or break it.
pub fn with_policy(page: &str) -> Result<String, Refusal> {
    if !page.contains(CHARSET) {
        return Err(Refusal::NoCharset);
    }
    if page.contains('\r') {
        return Err(Refusal::CarriageReturns);
    }
    if page.contains("<!--") {
        return Err(Refusal::Comments);
    }
    let lowered = page.to_ascii_lowercase();
    if ["<script", "</script", "<style", "</style"]
        .iter()
        .any(|tag| lowered.matches(tag).count() != page.matches(tag).count())
    {
        return Err(Refusal::TagsInCapitals);
    }
    let (scripts, styles) = (
        inline_bodies(page, "script")?,
        inline_bodies(page, "style")?,
    );
    let meta = format!(
        r#"<meta http-equiv="Content-Security-Policy" content="{}" />"#,
        policy(&scripts, &styles)
    );
    Ok(page.replacen(CHARSET, &format!("{CHARSET}\n    {meta}"), 1))
}

/// Nothing by default; the page's own scripts, its inline loader and wasm; its inline styles; its pictures from the
/// browser's memory; its font, and its wasm fetched, from its own address only.
fn policy(scripts: &[&str], styles: &[&str]) -> String {
    [
        "default-src 'none'".to_string(),
        directive(
            "script-src",
            &["'self'", "'wasm-unsafe-eval'"],
            hashes(scripts),
        ),
        directive("style-src", &[], hashes(styles)),
        "img-src blob: data:".to_string(),
        "font-src 'self'".to_string(),
        "connect-src 'self'".to_string(),
        "base-uri 'none'".to_string(),
        "form-action 'none'".to_string(),
    ]
    .join("; ")
}

/// A directive allowing its sources and hashes, or nothing at all when it has neither.
fn directive(name: &str, sources: &[&str], hashes: Vec<String>) -> String {
    let allowed: Vec<String> = sources
        .iter()
        .map(|source| source.to_string())
        .chain(hashes)
        .collect();
    if allowed.is_empty() {
        format!("{name} 'none'")
    } else {
        format!("{name} {}", allowed.join(" "))
    }
}

/// The SHA-256 of every inline body, as a policy names them.
fn hashes(bodies: &[&str]) -> Vec<String> {
    bodies
        .iter()
        .map(|body| {
            format!(
                "'sha256-{}'",
                STANDARD.encode(Sha256::digest(body.as_bytes()))
            )
        })
        .collect()
}

/// What every tag of a name holds between its opening and its closing, those that hold nothing left out, refused when
/// a tag's attribute holds a `>` or a tag is never closed.
fn inline_bodies<'page>(page: &'page str, tag: &str) -> Result<Vec<&'page str>, Refusal> {
    let (opening, closing) = (format!("<{tag}"), format!("</{tag}>"));
    let mut bodies = Vec::new();
    let mut rest = page;
    while let Some(at) = rest.find(&opening) {
        let after_the_name = &rest[at + opening.len()..];
        if !after_the_name.starts_with(|next: char| next == '>' || next.is_whitespace()) {
            rest = after_the_name;
            continue;
        }
        let end_of_the_tag = after_the_name.find('>').ok_or(Refusal::UnclosedTag)?;
        let attributes = &after_the_name[..end_of_the_tag];
        if ['"', '\'']
            .iter()
            .any(|quote| attributes.matches(*quote).count() % 2 == 1)
        {
            return Err(Refusal::AnglesInATag);
        }
        let held = &after_the_name[end_of_the_tag + 1..];
        let end = held.find(&closing).ok_or(Refusal::UnclosedTag)?;
        if end > 0 {
            bodies.push(&held[..end]);
        }
        rest = &held[end + closing.len()..];
    }
    Ok(bodies)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILT: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <style >body{color:red}</style>
    <style>p{margin:0}</style>
    <script type="module">import init from './x.js';</script>
    <script src="./elsewhere.js"></script>
  </head>
  <body><scripted-goblin>not a script</scripted-goblin></body>
</html>"#;

    #[test]
    fn a_built_page_gets_its_policy_right_after_its_charset_hashing_its_inline_script_and_style_only()
     {
        let page = with_policy(BUILT).expect("a page with a charset");

        let policy = concat!(
            r#"<meta charset="utf-8" />"#,
            "\n    ",
            r#"<meta http-equiv="Content-Security-Policy" content=""#,
            "default-src 'none'; ",
            "script-src 'self' 'wasm-unsafe-eval' 'sha256-MZ8Wk1XPE+P1CB59xWiRrLpBibGQ/fsEDVoANOH2aMo='; ",
            "style-src 'sha256-FcQqt3aNlV7AZnGV4zkQRVeCeJOxbMPnQSx258L803E=' 'sha256-QRQPRLGyxt3RJiKpYNjpkgHcmHU2PywsfL2e8oI0LQA='; ",
            "img-src blob: data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'",
            r#"" />"#,
        );
        assert_eq!(
            page,
            BUILT.replacen(r#"<meta charset="utf-8" />"#, policy, 1)
        );
    }

    #[test]
    fn a_page_with_no_inline_style_allows_no_style_at_all() {
        let page = BUILT
            .replace("<style >body{color:red}</style>", "")
            .replace("<style>p{margin:0}</style>", "");

        let protected = with_policy(&page);

        assert!(protected.is_ok_and(|page| page.contains("; style-src 'none';")));
    }

    #[test]
    fn a_page_without_its_charset_or_that_a_policy_could_protect_in_name_only_or_break_is_refused()
    {
        let pages = [
            BUILT.replace(r#"<meta charset="utf-8" />"#, ""),
            BUILT.replace('\n', "\r\n"),
            BUILT.replace("<head>", r#"<head><!-- <meta charset="utf-8" /> -->"#),
            BUILT.replace("<body>", "<body><!-- <script>alert(1)</script> -->"),
            BUILT.replace("<style>p{margin:0}</style>", "<STYLE>p{margin:0}</STYLE>"),
            BUILT.replace("<style>p{margin:0}</style>", "<style>p{margin:0}</Style>"),
            BUILT.replace(
                r#"<script type="module">"#,
                r#"<script type="module" data-goblin="a>b">"#,
            ),
            BUILT.replace(r#"<style >"#, r#"<style data-goblin='a>b'>"#),
            r#"<meta charset="utf-8" /><style>body{color:red}"#.to_string(),
            r#"<meta charset="utf-8" /><script type="module""#.to_string(),
        ];

        let refusals = pages.map(|page| with_policy(&page).err());

        assert_eq!(
            refusals,
            [
                Some(Refusal::NoCharset),
                Some(Refusal::CarriageReturns),
                Some(Refusal::Comments),
                Some(Refusal::Comments),
                Some(Refusal::TagsInCapitals),
                Some(Refusal::TagsInCapitals),
                Some(Refusal::AnglesInATag),
                Some(Refusal::AnglesInATag),
                Some(Refusal::UnclosedTag),
                Some(Refusal::UnclosedTag),
            ]
        );
    }
}
