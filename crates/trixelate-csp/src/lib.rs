//! The page's Content-Security-Policy, written into its built `index.html` after Trunk. It names the hashes of the
//! loader script and the styles Trunk writes inline, so the browser runs no other script and applies no other style,
//! and it lets the page fetch only from its own address: the picture dropped in never leaves the browser, and the
//! browser enforces it.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};

/// A built page with no `<meta charset="utf-8" />`, which the policy is written right after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoCharset;

const CHARSET: &str = r#"<meta charset="utf-8" />"#;

/// The page with its policy right after its charset, hashing every inline script and style it holds.
pub fn with_policy(page: &str) -> Result<String, NoCharset> {
    if !page.contains(CHARSET) {
        return Err(NoCharset);
    }
    let meta = format!(
        r#"<meta http-equiv="Content-Security-Policy" content="{}" />"#,
        policy(page)
    );
    Ok(page.replacen(CHARSET, &format!("{CHARSET}\n    {meta}"), 1))
}

/// Nothing by default; the page's own scripts, its inline loader and wasm; its inline styles; its pictures from the
/// browser's memory; its font, and its wasm fetched, from its own address only.
fn policy(page: &str) -> String {
    [
        "default-src 'none'".to_string(),
        directive(
            "script-src",
            &["'self'", "'wasm-unsafe-eval'"],
            hashes(page, "script"),
        ),
        directive("style-src", &[], hashes(page, "style")),
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

/// The SHA-256 of every inline body of a tag, as a policy names them.
fn hashes(page: &str, tag: &str) -> Vec<String> {
    inline_bodies(page, tag)
        .iter()
        .map(|body| {
            format!(
                "'sha256-{}'",
                STANDARD.encode(Sha256::digest(body.as_bytes()))
            )
        })
        .collect()
}

/// What every closed tag of a name holds between its opening and its closing, those that hold nothing left out.
fn inline_bodies<'page>(page: &'page str, tag: &str) -> Vec<&'page str> {
    let (opening, closing) = (format!("<{tag}"), format!("</{tag}>"));
    let mut bodies = Vec::new();
    let mut rest = page;
    while let Some(at) = rest.find(&opening) {
        let after_the_name = &rest[at + opening.len()..];
        if !after_the_name.starts_with(|next: char| next == '>' || next.is_whitespace()) {
            rest = after_the_name;
            continue;
        }
        let Some(end_of_the_tag) = after_the_name.find('>') else {
            break;
        };
        let held = &after_the_name[end_of_the_tag + 1..];
        let Some(end) = held.find(&closing) else {
            break;
        };
        if end > 0 {
            bodies.push(&held[..end]);
        }
        rest = &held[end + closing.len()..];
    }
    bodies
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
    fn a_page_without_its_charset_is_refused_and_an_unclosed_tag_hashes_nothing() {
        let without_charset = BUILT.replace(r#"<meta charset="utf-8" />"#, "");
        let unclosed = [
            r#"<meta charset="utf-8" /><style>body{color:red}"#,
            r#"<meta charset="utf-8" /><script type="module""#,
        ];

        let refused = with_policy(&without_charset);
        let policies = unclosed.map(|page| with_policy(page).unwrap_or_default());

        assert_eq!(refused, Err(NoCharset));
        assert!(
            policies.iter().all(
                |page| page.contains("script-src 'self' 'wasm-unsafe-eval'; style-src 'none';")
            ),
            "{policies:?}"
        );
    }
}
