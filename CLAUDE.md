# probeg

A CLI report of a Mac's "mileage": statistics about its life, not its specs. Rust, macOS only.
Run it, read it, exit: no background process, no stored state. Keep it that way.

- `src/collect/*`: one file per data source. Parsing is pure functions with tests. Any source
  may fail: the field becomes `None` and the report still draws.
- `src/model.rs` is the only link between collecting and drawing.
- `src/i18n/*`: one `Locale` per language. A missing string is a compile error.
- `src/fmt.rs`: numbers, sizes and dates per locale. `src/render/`: `theme` color, `layout` geometry, `table` rows and bars, `sections` content.
- `unsafe` lives only in `src/sys.rs` and `src/collect/smart.rs`.

Look: a plain Unix tool, like neofetch or htop. It must not read as AI-generated:
- Terminal's 16 colors only, color carries meaning (bar level, warnings). No truecolor, gradients or brand color.
- Aligned columns, short dry labels. No rules, boxes, dot leaders, icons, animation or spinners.
- No editorial voice: no jokes, personification or "takeaways". Report numbers.

Rules:
- Every user-visible string goes through `Locale`, in all languages at once.
- No page line wider than the window, in any language, at any width, with no data at all. Nothing clipped at normal widths. Tests in `render/mod.rs` enforce this.
- Never show serial numbers or arguments from shell history.
- Be honest about how far data goes back: "since power-on", "last 16 days", `≈` for estimates.
- Screenshots use `--demo`, never the real machine: `probeg --demo --force-color --width 110 > out.ansi`, then `scripts/ansi_to_html.py`.
- Everything: `scripts/check.sh`.
