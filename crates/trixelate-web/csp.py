"""Writes the page's Content-Security-Policy into its built index.html, after Trunk.

The policy names the hashes of the loader script and the styles Trunk writes inline, so the browser runs no other script
and applies no other style, and it lets the page fetch only from its own address: the picture dropped in never leaves
the browser, and the browser enforces it. Only release builds get it, as `trunk serve` adds its own reloading script.
"""

import base64
import hashlib
import os
import re

INLINE = r"<{tag}\b[^>]*>(.+?)</{tag}>"


def hashes(html, tag):
    return " ".join(
        "'sha256-" + base64.b64encode(hashlib.sha256(body.encode("utf-8")).digest()).decode() + "'"
        for body in re.findall(INLINE.format(tag=tag), html, re.S)
    )


def policy(html):
    return "; ".join(
        [
            "default-src 'none'",
            f"script-src 'self' 'wasm-unsafe-eval' {hashes(html, 'script')}",
            f"style-src {hashes(html, 'style')}",
            "img-src blob: data:",
            "font-src 'self'",
            "connect-src 'self'",
            "base-uri 'none'",
            "form-action 'none'",
        ]
    )


if os.environ.get("TRUNK_PROFILE") == "release":
    page = os.path.join(os.environ["TRUNK_STAGING_DIR"], "index.html")
    with open(page, encoding="utf-8") as built:
        html = built.read()
    meta = f'<meta http-equiv="Content-Security-Policy" content="{policy(html)}" />'
    charset = '<meta charset="utf-8" />'
    if charset not in html:
        raise SystemExit("index.html lost its charset, which the policy is written after")
    with open(page, "w", encoding="utf-8") as built:
        built.write(html.replace(charset, f"{charset}\n    {meta}", 1))
