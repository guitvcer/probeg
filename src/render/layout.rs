//! Geometry of strings with color codes: width, alignment, columns.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Visible width of a string, ignoring escape sequences.
pub fn width(text: &str) -> usize {
    if !text.contains('\x1b') {
        return text.width();
    }
    let mut total = 0;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for code in chars.by_ref() {
                if code.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            total += c.width().unwrap_or(0);
        }
    }
    total
}

/// Pads with spaces to the target width.
pub fn pad(text: &str, target: usize) -> String {
    let current = width(text);
    format!("{text}{}", " ".repeat(target.saturating_sub(current)))
}

/// Truncates uncolored text with an ellipsis.
pub fn fit(text: &str, max: usize) -> String {
    if text.width() <= max {
        return text.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w + 1 > max {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push('…');
    out
}

/// Wraps uncolored text on spaces. A run wider than the line, which is every
/// Chinese or Japanese sentence, breaks between characters.
pub fn wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.width() + 1 + word.width() <= max {
            current.push(' ');
            current.push_str(word);
            continue;
        }
        if !current.is_empty() && word.width() <= max {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        for c in word.chars() {
            if current.width() + c.width().unwrap_or(0) > max {
                lines.push(std::mem::take(&mut current));
            }
            current.push(c);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Lays blocks out in two columns: each next one goes into the shorter column.
pub fn masonry(blocks: Vec<Vec<String>>, column: usize, gap: usize) -> Vec<String> {
    let mut left: Vec<String> = Vec::new();
    let mut right: Vec<String> = Vec::new();
    for block in blocks {
        let target = if left.len() <= right.len() {
            &mut left
        } else {
            &mut right
        };
        if !target.is_empty() {
            target.push(String::new());
        }
        target.extend(block);
    }
    let rows = left.len().max(right.len());
    (0..rows)
        .map(|i| {
            let l = left.get(i).map_or("", String::as_str);
            let r = right.get(i).map_or("", String::as_str);
            if r.is_empty() {
                l.to_string()
            } else {
                format!("{}{}{r}", pad(l, column), " ".repeat(gap))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_ignores_colour_codes() {
        assert_eq!(width("\x1b[1m\x1b[38;2;1;2;3mCafé\x1b[0m"), 4);
        assert_eq!(width("plain"), 5);
    }

    #[test]
    fn fit_respects_limit() {
        assert_eq!(fit("café", 20), "café");
        assert_eq!(fit("naïve long name", 6), "naïve…");
        assert_eq!(fit("abc", 0), "");
    }

    #[test]
    fn wrap_breaks_on_words() {
        assert_eq!(wrap("café bar tea", 8), ["café bar", "tea"]);
        assert_eq!(wrap("", 8), Vec::<String>::new());
        assert_eq!(wrap("内存吃紧交换区", 6), ["内存吃", "紧交换", "区"]);
        assert_eq!(wrap("ok 内存吃紧", 6), ["ok 内", "存吃紧"]);
        for line in wrap("überlongwordthatwillnotfitontheline ok", 10) {
            assert!(line.width() <= 10, "{line}");
        }
    }

    #[test]
    fn masonry_balances_columns() {
        let blocks = vec![
            vec!["a".to_string(); 5],
            vec!["b".to_string(); 2],
            vec!["c".to_string(); 2],
        ];
        let rows = masonry(blocks, 3, 2);
        assert_eq!(rows[0], "a    b");
        assert_eq!(rows[2], "a");
        assert_eq!(rows[3], "a    c");
    }
}
