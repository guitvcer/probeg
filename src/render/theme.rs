//! Color. Only the terminal's own 16-color palette, so the report takes on the
//! user's theme like any other command-line tool.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tone {
    Green,
    Yellow,
    Red,
}

#[derive(Clone, Copy, Debug)]
pub struct Painter {
    pub color: bool,
}

impl Painter {
    pub fn detect() -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        Self {
            color: !no_color && crate::sys::stdout_is_tty(),
        }
    }

    fn sgr(&self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn bold(&self, text: &str) -> String {
        self.sgr("1", text)
    }

    pub fn dim(&self, text: &str) -> String {
        self.sgr("2", text)
    }

    pub fn tone(&self, tone: Tone, text: &str) -> String {
        let code = match tone {
            Tone::Green => "32",
            Tone::Yellow => "33",
            Tone::Red => "31",
        };
        self.sgr(code, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_mode_emits_no_escapes() {
        let p = Painter { color: false };
        assert_eq!(p.bold("x"), "x");
        assert_eq!(p.dim("x"), "x");
        assert_eq!(p.tone(Tone::Red, "x"), "x");
    }

    #[test]
    fn color_uses_the_terminal_palette() {
        let p = Painter { color: true };
        assert_eq!(p.tone(Tone::Red, "x"), "\x1b[31mx\x1b[0m");
    }
}
