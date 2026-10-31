#!/usr/bin/env python3
"""Turn probeg's colored output into an HTML page for README screenshots.

Usage: ansi_to_html.py INPUT OUTPUT [--bg COLOR] [--fg COLOR]

Browsers don't keep CJK text on the character grid the way terminals do, so every
wide character gets a fixed two-cell box.
"""
import argparse
import html
import re
import unicodedata

ESCAPE = re.compile(r"\x1b\[([0-9;?]*)([A-Za-z])")
# Terminal.app's default ANSI colors, for a screenshot that looks like the real thing.
PALETTE = {"31": "194,54,33", "32": "37,188,36", "33": "173,173,39"}
DIM = "128,128,128"


def cell(char):
    if unicodedata.east_asian_width(char) in ("W", "F"):
        return f'<span class="wide">{html.escape(char)}</span>'
    return html.escape(char)


def convert(text):
    out, pos = [], 0
    fg = bg = None
    bold = False

    def open_span():
        style = []
        if fg:
            style.append(f"color:rgb({fg})")
        if bg:
            style.append(f"background:rgb({bg})")
        if bold:
            style.append("font-weight:700")
        return f'<span style="{";".join(style)}">'

    out.append(open_span())
    for match in ESCAPE.finditer(text):
        out.extend(cell(c) for c in text[pos : match.start()])
        pos = match.end()
        if match.group(2) != "m":
            continue
        code = match.group(1)
        if code in ("0", ""):
            fg = bg = None
            bold = False
        elif code == "1":
            bold = True
        elif code == "2":
            fg = DIM
        elif code in PALETTE:
            fg = PALETTE[code]
        out.append("</span>" + open_span())
    out.extend(cell(c) for c in text[pos:])
    out.append("</span>")
    return "".join(out)


PAGE = """<!doctype html><meta charset="utf-8"><style>
body {{ margin: 0; background: {bg}; color: {fg}; display: inline-block; }}
pre {{ font: 13px/1.25 Menlo, "Hiragino Sans", "PingFang SC", "Apple SD Gothic Neo", monospace;
  margin: 0; padding: 14px 18px; }}
.wide {{ display: inline-block; width: 2ch; text-align: center; }}
</style><pre>{body}</pre>"""


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input")
    parser.add_argument("output")
    parser.add_argument("--bg", default="#1e1e1e")
    parser.add_argument("--fg", default="#d4d4d4")
    args = parser.parse_args()
    with open(args.input, encoding="utf-8") as source:
        body = convert(source.read().rstrip("\n"))
    with open(args.output, "w", encoding="utf-8") as target:
        target.write(PAGE.format(bg=args.bg, fg=args.fg, body=body))


if __name__ == "__main__":
    main()
