//! A section is a heading and aligned columns: label, value, optional note.

use super::layout::{fit, pad, width};
use super::theme::{Painter, Tone};

const BAR_WIDTH: usize = 30;
const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

enum Item {
    Row {
        label: String,
        value: String,
        note: String,
        tone: Option<Tone>,
    },
    Bar {
        share: f64,
        tone: Tone,
        text: String,
    },
    Text(String),
}

pub struct Section {
    title: String,
    items: Vec<Item>,
}

impl Section {
    pub fn new(title: &str) -> Self {
        let mut chars = title.chars();
        let title = chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(chars).collect()
        });
        Self {
            title,
            items: Vec::new(),
        }
    }

    pub fn row(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.note(label, value, "");
    }

    pub fn note(
        &mut self,
        label: impl Into<String>,
        value: impl Into<String>,
        note: impl Into<String>,
    ) {
        self.items.push(Item::Row {
            label: label.into(),
            value: value.into(),
            note: note.into(),
            tone: None,
        });
    }

    pub fn toned(
        &mut self,
        label: impl Into<String>,
        value: impl Into<String>,
        note: impl Into<String>,
        tone: Tone,
    ) {
        self.items.push(Item::Row {
            label: label.into(),
            value: value.into(),
            note: note.into(),
            tone: Some(tone),
        });
    }

    pub fn bar(&mut self, share: f64, tone: Tone, text: impl Into<String>) {
        self.items.push(Item::Bar {
            share,
            tone,
            text: text.into(),
        });
    }

    pub fn text(&mut self, text: impl Into<String>) {
        self.items.push(Item::Text(text.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn render(&self, p: &Painter, column: usize) -> Vec<String> {
        let (mut labels, mut values) = (0, 0);
        for item in &self.items {
            if let Item::Row {
                label, value, note, ..
            } = item
            {
                labels = labels.max(width(label));
                // Only rows with a note need the value column kept short.
                if !note.is_empty() {
                    values = values.max(width(value));
                }
            }
        }
        let labels = labels.min(column * 3 / 5);
        let values = values.min(column.saturating_sub(labels + 2));

        let mut out = vec![p.bold(&fit(&self.title, column))];
        for item in &self.items {
            out.push(match item {
                Item::Row {
                    label,
                    value,
                    note,
                    tone,
                } => {
                    let label = pad(&fit(label, labels), labels);
                    let room = column.saturating_sub(labels + values + 4);
                    if note.is_empty() || width(note) > room {
                        let value = fit(value, column.saturating_sub(labels + 2));
                        let painted =
                            tone.map_or_else(|| value.clone(), |tone| p.tone(tone, &value));
                        format!("{label}  {painted}")
                    } else {
                        let value = fit(value, values);
                        let painted =
                            tone.map_or_else(|| value.clone(), |tone| p.tone(tone, &value));
                        format!("{label}  {}  {}", pad(&painted, values), p.dim(note))
                    }
                }
                Item::Bar { share, tone, text } => bar(p, *share, *tone, text, column),
                Item::Text(text) => p.dim(&fit(text, column)),
            });
        }
        out
    }
}

/// htop-style meter: `[||||||||        ] 41%`.
fn bar(p: &Painter, share: f64, tone: Tone, text: &str, column: usize) -> String {
    let cells = BAR_WIDTH.min(column.saturating_sub(width(text) + 3));
    let filled = (share.clamp(0.0, 1.0) * cells as f64).round() as usize;
    format!(
        "[{}{}] {text}",
        p.tone(tone, &"|".repeat(filled)),
        " ".repeat(cells - filled)
    )
}

/// One block per value, scaled to the largest.
pub fn sparkline(values: &[f64]) -> String {
    let max = values.iter().copied().fold(0.0, f64::max);
    values
        .iter()
        .map(|value| {
            if max <= 0.0 {
                return BLOCKS[0];
            }
            BLOCKS[((value / max) * (BLOCKS.len() - 1) as f64).round() as usize]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> Painter {
        Painter { color: false }
    }

    #[test]
    fn columns_line_up() {
        let mut s = Section::new("Battery");
        s.row("cycles", "183");
        s.note("capacity", "97%", "new");
        let lines = s.render(&plain(), 40);
        assert_eq!(lines, ["Battery", "cycles    183", "capacity  97%  new"]);
    }

    #[test]
    fn notes_are_dropped_before_anything_overflows() {
        let mut s = Section::new("x");
        s.note("label", "value", "a note far too long for this column");
        let lines = s.render(&plain(), 20);
        assert_eq!(lines[1], "label  value");
        let mut s = Section::new("x");
        s.note("a", "1", "n");
        s.row("b", "a value wider than the column of notes");
        let lines = s.render(&plain(), 60);
        assert_eq!(lines[1], "a  1  n");
    }

    #[test]
    fn oversized_rows_fit_the_column() {
        let mut s = Section::new("A very long title that does not fit");
        s.row(
            "a very long label that does not fit",
            "and a very long value too",
        );
        for line in s.render(&plain(), 24) {
            assert!(width(&line) <= 24, "{line}");
        }
    }

    #[test]
    fn bar_fills_by_share() {
        assert_eq!(
            bar(&plain(), 0.5, Tone::Green, "50%", 16),
            "[|||||     ] 50%"
        );
        assert_eq!(bar(&plain(), 2.0, Tone::Green, "x", 7), "[|||] x");
        assert_eq!(bar(&plain(), 0.0, Tone::Green, "x", 7), "[   ] x");
    }

    #[test]
    fn sparkline_scales_to_the_peak() {
        assert_eq!(sparkline(&[0.0, 5.0, 10.0]), "▁▅█");
        assert_eq!(sparkline(&[0.0, 0.0]), "▁▁");
    }
}
