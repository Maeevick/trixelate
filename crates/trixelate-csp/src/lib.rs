//! The page's Content-Security-Policy, written into its built `index.html` after Trunk. It names the hashes of the
//! loader script and the styles Trunk writes inline, so the browser runs no other script and applies no other style,
//! and it lets the page fetch only from its own address: the picture dropped in never leaves the browser, and the
//! browser enforces it.

/// A built page with no `<meta charset="utf-8" />`, which the policy is written right after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoCharset;

/// The page with its policy right after its charset, hashing every inline script and style it holds.
pub fn with_policy(page: &str) -> Result<String, NoCharset> {
    Ok(page.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILT: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <style >body{color:red}</style>
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
            "style-src 'sha256-FcQqt3aNlV7AZnGV4zkQRVeCeJOxbMPnQSx258L803E='; ",
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
            r#"<meta charset="utf-8" /><script"#,
        ];

        let refused = with_policy(&without_charset);
        let policies = unclosed.map(|page| with_policy(page).unwrap_or_default());

        assert_eq!(refused, Err(NoCharset));
        assert!(
            policies
                .iter()
                .all(|page| page.contains("script-src 'self' 'wasm-unsafe-eval'; style-src ;")),
            "{policies:?}"
        );
    }
}
