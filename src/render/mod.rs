//! The page: one bold name, one line of specs, then sections in columns.

mod layout;
mod sections;
mod table;
pub mod theme;

use crate::i18n::Locale;
use crate::model::Report;
use layout::{masonry, wrap};
use theme::Painter;

const MAX_WIDTH: usize = 110;
const MIN_WIDTH: usize = 40;
const GAP: usize = 4;
/// Below this, two columns squeeze the notes out of the rows.
const TWO_COLUMNS_FROM: usize = 96;

#[derive(Clone, Copy)]
pub struct Options {
    pub width: usize,
    pub painter: Painter,
    pub locale: &'static Locale,
}

/// The whole page as text.
pub fn page(r: &Report, options: Options) -> String {
    let (p, l) = (&options.painter, options.locale);
    let content = options.width.clamp(MIN_WIDTH, MAX_WIDTH);
    let mut lines = Vec::new();
    if let Some(name) = r
        .identity
        .computer_name
        .as_ref()
        .or(r.identity.model.as_ref())
    {
        lines.push(p.bold(name));
    }
    lines.extend(wrap(&sections::spec_line(r, l), content));
    lines.push(String::new());

    let two_columns = content >= TWO_COLUMNS_FROM;
    let column = if two_columns {
        (content - GAP) / 2
    } else {
        content
    };
    let blocks: Vec<Vec<String>> = sections::all(r, l)
        .iter()
        .map(|section| section.render(p, column))
        .collect();
    if two_columns {
        lines.extend(masonry(blocks, column, GAP));
    } else {
        lines.extend(blocks.join(&String::new()));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::layout::width;
    use super::*;
    use crate::i18n::Lang;
    use crate::model::UsageStatus;
    use crate::sample;

    fn options(w: usize, color: bool, lang: Lang) -> Options {
        Options {
            width: w,
            painter: Painter { color },
            locale: lang.locale(),
        }
    }

    fn assert_fits(report: &Report, w: usize, color: bool, lang: Lang) {
        for line in page(report, options(w, color, lang)).lines() {
            assert!(
                width(line) <= w.max(MIN_WIDTH),
                "{lang:?}, width {w}, {}: {line:?}",
                width(line)
            );
        }
    }

    #[test]
    fn no_line_overflows_in_any_language_at_any_width() {
        for lang in Lang::ALL {
            let report = sample::report();
            for w in [40, 60, 80, 95, 96, 110, 160, 300] {
                assert_fits(&report, w, false, lang);
                assert_fits(&report, w, true, lang);
            }
        }
    }

    #[test]
    fn nothing_is_cut_off_at_a_normal_width() {
        for lang in Lang::ALL {
            for w in [80, 120] {
                for line in page(&sample::report(), options(w, false, lang)).lines() {
                    assert!(!line.contains('…'), "{lang:?} at {w}: {line}");
                }
            }
        }
    }

    #[test]
    fn report_with_no_data_still_renders() {
        for lang in Lang::ALL {
            let report = Report {
                generated_at: sample::report().generated_at,
                ..Report::default()
            };
            for w in [40, 80, 120] {
                assert_fits(&report, w, false, lang);
            }
        }
    }

    #[test]
    fn locked_usage_explains_how_to_open_it() {
        let mut report = sample::report();
        report.usage = Default::default();
        report.usage.status = UsageStatus::NeedsFullDiskAccess;
        let text = page(&report, options(120, false, Lang::En));
        assert!(text.contains("locked"), "{text}");
    }

    #[test]
    fn page_shows_what_the_report_is_for() {
        let text = page(&sample::report(), options(120, false, Lang::En));
        for needle in [
            "47.1 TB",
            "183 / 1,000",
            "97%",
            "May 2025",
            "Jul 2, 2025",
            "Oct 18",
            "Terminal",
            "Battery",
            "Memory",
        ] {
            assert!(text.contains(needle), "missing {needle}:\n{text}");
        }
    }

    #[test]
    fn languages_do_not_leak_into_each_other() {
        let en = page(&sample::report(), options(120, false, Lang::En));
        assert!(!en.chars().any(|c| ('а'..='я').contains(&c)), "{en}");
        let ru = page(&sample::report(), options(120, false, Lang::Ru));
        assert!(ru.contains("Батарея") && ru.contains("183 / 1 000"), "{ru}");
    }

    #[test]
    fn no_placeholder_reaches_the_screen() {
        for lang in Lang::ALL {
            let text = page(&sample::report(), options(120, false, lang));
            assert!(!text.contains('{'), "{lang:?}: {text}");
        }
    }

    #[test]
    fn plain_page_has_no_escape_codes() {
        assert!(!page(&sample::report(), options(100, false, Lang::En)).contains('\x1b'));
    }

    #[test]
    fn colors_come_from_the_terminal_palette_only() {
        let text = page(&sample::report(), options(100, true, Lang::En));
        assert!(!text.contains("38;2;") && !text.contains("48;2;"));
    }

    #[test]
    fn serial_numbers_never_reach_the_report() {
        let json = serde_json::to_string(&sample::report()).unwrap();
        assert!(!json.to_lowercase().contains("serial"));
    }
}
