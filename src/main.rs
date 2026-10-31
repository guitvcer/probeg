use clap::{ArgAction, CommandFactory, FromArgMatches, Parser};

use probeg::i18n::{Lang, Locale};
use probeg::render::theme::Painter;
use probeg::render::{self, Options};
use probeg::{collect, sample, sys};

const FALLBACK_WIDTH: usize = 100;

#[derive(Parser)]
#[command(
    name = "probeg",
    version,
    disable_help_flag = true,
    disable_version_flag = true
)]
struct Cli {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    plain: bool,
    #[arg(long)]
    demo: bool,
    #[arg(long, value_name = "CODE", value_parser = parse_lang)]
    lang: Option<Lang>,
    #[arg(short = 'h', long, action = ArgAction::Help)]
    help: Option<bool>,
    #[arg(short = 'V', long, action = ArgAction::Version)]
    version: Option<bool>,
    /// Page width in characters (defaults to the window's).
    #[arg(long, hide = true)]
    width: Option<usize>,
    /// Color even when writing to a file.
    #[arg(long, hide = true)]
    force_color: bool,
}

fn language_list() -> String {
    Lang::ALL
        .iter()
        .map(|lang| lang.locale().code)
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_lang(tag: &str) -> Result<Lang, String> {
    Lang::parse(tag)
        .ok_or_else(|| format!("unknown language '{tag}', available: {}", language_list()))
}

/// `--help` has to be translated before clap parses anything, so `--lang` is
/// looked up by hand first. Invalid values are left for clap to report.
fn language_from_args(args: &[String]) -> Option<Lang> {
    args.iter()
        .enumerate()
        .find_map(|(i, arg)| match arg.strip_prefix("--lang") {
            Some(rest) if rest.starts_with('=') => Lang::parse(&rest[1..]),
            Some("") => args.get(i + 1).and_then(|value| Lang::parse(value)),
            _ => None,
        })
}

fn localized_command(l: &Locale) -> clap::Command {
    let languages = format!("{} ({})", l.cli_lang, language_list());
    Cli::command()
        .about(l.cli_about)
        .help_template(format!(
            "{{about}}\n\n{}: {{usage}}\n\n{}:\n{{options}}\n",
            l.cli_usage, l.cli_options
        ))
        .mut_arg("json", |arg| arg.help(l.cli_json))
        .mut_arg("plain", |arg| arg.help(l.cli_plain))
        .mut_arg("demo", |arg| arg.help(l.cli_demo))
        .mut_arg("lang", |arg| arg.help(languages))
        .mut_arg("help", |arg| arg.help(l.cli_help))
        .mut_arg("version", |arg| arg.help(l.cli_version))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let lang = language_from_args(&args).unwrap_or_else(Lang::current);
    let matches = localized_command(lang.locale()).get_matches_from(&args);
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    show(&cli, cli.lang.unwrap_or(lang).locale());
}

fn show(cli: &Cli, locale: &'static Locale) {
    let report = if cli.demo {
        sample::report()
    } else {
        collect::collect()
    };
    if cli.json {
        match serde_json::to_string_pretty(&report) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    let painter = if cli.plain {
        Painter { color: false }
    } else if cli.force_color {
        Painter { color: true }
    } else {
        Painter::detect()
    };
    let width = cli
        .width
        .or_else(sys::terminal_width)
        .unwrap_or(FALLBACK_WIDTH);
    print!(
        "{}",
        render::page(
            &report,
            Options {
                width,
                painter,
                locale
            }
        )
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn language_flag_is_found_before_parsing() {
        assert_eq!(
            language_from_args(&args(&["probeg", "--lang", "ja"])),
            Some(Lang::Ja)
        );
        assert_eq!(
            language_from_args(&args(&["probeg", "--lang=pt-BR"])),
            Some(Lang::Pt)
        );
        assert_eq!(language_from_args(&args(&["probeg", "--lang", "xx"])), None);
        assert_eq!(language_from_args(&args(&["probeg", "--json"])), None);
    }

    #[test]
    fn every_language_builds_a_valid_command() {
        for lang in Lang::ALL {
            localized_command(lang.locale()).debug_assert();
        }
    }

    #[test]
    fn unknown_language_names_the_options() {
        let error = parse_lang("klingon").unwrap_err();
        assert!(error.contains("ru") && error.contains("zh"), "{error}");
    }
}
