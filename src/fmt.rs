//! Numbers, sizes, durations and dates, formatted the way each language writes them.

use chrono::{Datelike, Local, TimeZone, Timelike};

use crate::i18n::{Forms, Locale, Plural};
use crate::model::Ts;

pub fn from_unix(secs: i64) -> Option<Ts> {
    Local.timestamp_opt(secs, 0).single()
}

impl Locale {
    fn form<'a>(&self, n: u64, forms: &'a Forms) -> &'a str {
        match self.plural {
            Plural::None => forms.many,
            Plural::OneOther if n == 1 => forms.one,
            Plural::ZeroOneOther if n <= 1 => forms.one,
            Plural::OneOther | Plural::ZeroOneOther => forms.many,
            Plural::Slavic => match (n % 10, n % 100) {
                (_, 11..=14) => forms.many,
                (1, _) => forms.one,
                (2..=4, _) => forms.few,
                _ => forms.many,
            },
        }
    }

    /// A number with its word in the right form: «183 цикла», «3次».
    pub fn count(&self, n: u64, forms: &Forms) -> String {
        let space = if self.unit_space || self.plural != Plural::None {
            " "
        } else {
            ""
        };
        format!("{}{space}{}", self.group(n), self.form(n, forms))
    }

    /// 47102 → «47,102», «47 102» or «47.102».
    pub fn group(&self, n: u64) -> String {
        let digits = n.to_string();
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                out.push(self.group_separator);
            }
            out.push(c);
        }
        out
    }

    pub fn decimal(&self, value: f64, places: usize) -> String {
        format!("{value:.places$}").replace('.', &self.decimal_separator.to_string())
    }

    /// Decimal bytes, the way macOS itself counts: «47.1 TB».
    pub fn bytes(&self, n: u64) -> String {
        let units = [self.mb, self.gb, self.tb, self.pb];
        let mut value = n as f64 / 1e6;
        if value < 1.0 {
            return format!("{} {}", (n as f64 / 1e3).round() as u64, self.kb);
        }
        let mut unit = 0;
        while value >= 999.5 && unit < units.len() - 1 {
            value /= 1000.0;
            unit += 1;
        }
        let places = if value < 10.0 {
            2
        } else if value < 100.0 {
            1
        } else {
            0
        };
        format!("{} {}", self.decimal(value, places), units[unit])
    }

    /// RAM is counted in binary gigabytes.
    pub fn ram(&self, n: u64) -> String {
        let gib = n as f64 / (1u64 << 30) as f64;
        if gib >= 10.0 {
            format!("{} {}", self.decimal(gib, 0), self.gb)
        } else if gib >= 1.0 {
            format!("{} {}", self.decimal(gib, 1), self.gb)
        } else {
            format!(
                "{} {}",
                (n as f64 / (1u64 << 20) as f64).round() as u64,
                self.mb
            )
        }
    }

    fn unit(&self, value: u64, unit: &str) -> String {
        if self.unit_space {
            format!("{value} {unit}")
        } else {
            format!("{value}{unit}")
        }
    }

    /// The two largest time units: «12d 8h», «3 ч 12 мин», «40秒».
    pub fn duration(&self, secs: f64) -> String {
        let s = secs.max(0.0) as u64;
        let (d, h, m) = (s / 86_400, s % 86_400 / 3600, s % 3600 / 60);
        let (day, hour, minute) = (
            self.unit(d, self.unit_day),
            self.unit(h, self.unit_hour),
            self.unit(m, self.unit_minute),
        );
        match (d, h, m) {
            (0, 0, 0) => self.unit(s, self.unit_second),
            (0, 0, _) => minute,
            (0, _, 0) => hour,
            (0, _, _) => format!("{hour} {minute}"),
            (_, 0, _) => day,
            _ => format!("{day} {hour}"),
        }
    }

    /// One decimal and an hour unit, for the app ranking.
    pub fn hours(&self, secs: f64) -> String {
        let hours = self.decimal(secs / 3600.0, 1);
        if self.unit_space {
            format!("{hours} {}", self.unit_hour)
        } else {
            format!("{hours}{}", self.unit_hour)
        }
    }

    /// Calendar age: «1 year 4 mo».
    pub fn age(&self, from: Ts, to: Ts) -> String {
        let mut months = (to.year() - from.year()) * 12 + to.month() as i32 - from.month() as i32;
        if to.day() < from.day() {
            months -= 1;
        }
        let months = months.max(0) as u64;
        let (years, rest) = (months / 12, months % 12);
        let year_text = self.count(years, &self.years);
        let month_text = self.count(rest, &self.months_count);
        match (years, rest) {
            (0, 0) => self.count((to - from).num_days().max(0) as u64, &self.days),
            (0, _) => month_text,
            (_, 0) => year_text,
            _ => format!("{year_text} {month_text}"),
        }
    }

    pub fn date(&self, t: Ts) -> String {
        self.date_with(self.date_format, self.months[t.month0() as usize], t)
    }

    pub fn month_year(&self, t: Ts) -> String {
        self.date_with(
            self.month_year_format,
            self.months_full[t.month0() as usize],
            t,
        )
    }

    pub fn date_time(&self, t: Ts) -> String {
        self.date_with(self.date_time_format, self.months[t.month0() as usize], t)
    }

    fn date_with(&self, format: &str, month: &str, t: Ts) -> String {
        format
            .replace("{d}", &t.day().to_string())
            .replace("{m}", month)
            .replace("{y}", &t.year().to_string())
            .replace("{t}", &format!("{:02}:{:02}", t.hour(), t.minute()))
    }
}

#[cfg(test)]
mod tests {
    use crate::i18n::Lang;
    use crate::model::Ts;
    use chrono::{Local, TimeZone};

    fn at(y: i32, mo: u32, d: u32) -> Ts {
        Local.with_ymd_and_hms(y, mo, d, 12, 0, 0).unwrap()
    }

    #[test]
    fn russian_plurals_follow_the_last_digit() {
        let ru = Lang::Ru.locale();
        for (n, want) in [
            (1, "1 день"),
            (2, "2 дня"),
            (4, "4 дня"),
            (5, "5 дней"),
            (11, "11 дней"),
            (14, "14 дней"),
            (21, "21 день"),
            (22, "22 дня"),
            (111, "111 дней"),
        ] {
            assert_eq!(ru.count(n, &ru.days), want);
        }
    }

    #[test]
    fn other_plural_rules() {
        let en = Lang::En.locale();
        assert_eq!(en.count(0, &en.days), "0 days");
        assert_eq!(en.count(1, &en.days), "1 day");
        let fr = Lang::Fr.locale();
        assert_eq!(fr.count(0, &fr.days), "0 jour");
        assert_eq!(fr.count(2, &fr.days), "2 jours");
        let zh = Lang::Zh.locale();
        assert_eq!(zh.count(3, &zh.days), "3天");
        let ko = Lang::Ko.locale();
        assert_eq!(ko.count(3, &ko.days), "3일");
    }

    #[test]
    fn numbers_use_local_separators() {
        assert_eq!(Lang::En.locale().group(1234567), "1,234,567");
        assert_eq!(Lang::Ru.locale().group(47102), "47 102");
        assert_eq!(Lang::De.locale().group(47102), "47.102");
        assert_eq!(Lang::En.locale().group(999), "999");
        assert_eq!(Lang::En.locale().group(0), "0");
    }

    #[test]
    fn bytes_are_decimal_and_rounded_by_size() {
        let en = Lang::En.locale();
        assert_eq!(en.bytes(512_000), "512 KB");
        assert_eq!(en.bytes(5_100_000), "5.10 MB");
        assert_eq!(en.bytes(66_921_412_061), "66.9 GB");
        assert_eq!(en.bytes(394_000_000_000), "394 GB");
        assert_eq!(en.bytes(999_600_000), "1.00 GB");
        assert_eq!(Lang::Ru.locale().bytes(47_100_000_000_000), "47,1 ТБ");
        assert_eq!(Lang::Fr.locale().bytes(47_100_000_000_000), "47,1 To");
    }

    #[test]
    fn ram_is_binary() {
        let en = Lang::En.locale();
        assert_eq!(en.ram(17_179_869_184), "16 GB");
        assert_eq!(en.ram(3 << 29), "1.5 GB");
        assert_eq!(en.ram(300 << 20), "300 MB");
    }

    #[test]
    fn duration_shows_two_units() {
        let en = Lang::En.locale();
        assert_eq!(en.duration(40.0), "40s");
        assert_eq!(en.duration(300.0), "5m");
        assert_eq!(en.duration(3600.0), "1h");
        assert_eq!(en.duration(3600.0 * 3.0 + 720.0), "3h 12m");
        assert_eq!(
            en.duration(86_400.0 * 12.0 + 3600.0 * 8.0 + 120.0),
            "12d 8h"
        );
        assert_eq!(en.duration(86_400.0 * 2.0 + 60.0), "2d");
        assert_eq!(en.duration(-5.0), "0s");
        assert_eq!(
            Lang::Ru.locale().duration(3600.0 * 3.0 + 720.0),
            "3 ч 12 мин"
        );
        assert_eq!(Lang::Zh.locale().duration(86_400.0 + 3600.0), "1天 1小时");
    }

    #[test]
    fn age_is_calendar_based() {
        let en = Lang::En.locale();
        assert_eq!(en.age(at(2025, 7, 2), at(2026, 10, 31)), "1 year 3 mo");
        assert_eq!(en.age(at(2025, 7, 2), at(2027, 7, 2)), "2 years");
        assert_eq!(en.age(at(2026, 10, 1), at(2026, 10, 31)), "30 days");
        assert_eq!(en.age(at(2026, 1, 31), at(2026, 3, 1)), "1 mo");
    }

    #[test]
    fn dates_read_locally() {
        let day = at(2025, 7, 2);
        assert_eq!(Lang::En.locale().date(day), "Jul 2, 2025");
        assert_eq!(Lang::Ru.locale().date(day), "2 июл 2025");
        assert_eq!(Lang::De.locale().date(day), "2. Juli 2025");
        assert_eq!(Lang::Zh.locale().date(day), "2025年7月2日");
        assert_eq!(Lang::Ko.locale().date(day), "2025년 7월 2일");
        assert_eq!(Lang::Es.locale().month_year(day), "julio de 2025");
        assert_eq!(Lang::Ja.locale().month_year(day), "2025年7月");
        assert_eq!(Lang::En.locale().date_time(day), "Jul 2, 12:00");
    }
}
