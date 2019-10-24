use super::facade;
use boolinator::Boolinator;
use clap::{App, AppSettings, Arg, SubCommand};

fn range_validator(low: u32, up: u32) -> Box<dyn Fn(String) -> Result<(), String>> {
    Box::new(move |num: String| match num.parse::<u32>() {
        Err(_) => Err(String::from(
            "please specify unsigned integer after '~contest'.",
        )),
        Ok(num) if num == low => Err(String::from("too small number.")),
        Ok(num) if num > up => Err(String::from("too large number.")),
        Ok(_) => Ok(()),
    })
}

fn parse_validator<T: std::str::FromStr>(num: String) -> Result<(), String> {
    num.parse::<T>()
        .map(|_| ())
        .map_err(|_| format!("`{}` is invalid.", num))
}

fn language_validator(language: String) -> Result<(), String> {
    if !facade::QUIZ_COMMANDS_REGEX.is_match(&language) {
        Err(format!("unexpected language '{}'.", language))
    } else {
        Ok(())
    }
}

fn prefix_validator(prefix: String) -> Result<(), String> {
    (!prefix.is_ascii() || prefix.len() <= 5).as_result(
        (),
        "Please specify an ASCII string that less than or equal to 5 characters".into(),
    )
}

pub(crate) fn contest(
    args: &mut serenity::framework::standard::Args,
) -> clap::Result<(u32, Vec<String>)> {
    App::new("contest")
        .version("0.0.1")
        .setting(AppSettings::ColorNever)
        .arg(
            Arg::with_name("number")
                .required(true)
                .validator(range_validator(1, 100))
                .help("Number of contest problems"),
        )
        .arg(
            Arg::with_name("languages")
                .required(true)
                .use_delimiter(true)
                .validator(language_validator)
                .takes_value(true)
                .default_value(facade::QUIZ_COMMANDS.to_vec().join(",").as_str())
                .min_values(1)
                .help("List of contest languages"),
        )
        .get_matches_from_safe(
            std::iter::once("contest".to_string())
                .chain(args.iter::<String>().filter_map(Result::ok)),
        )
        .map(|matches| {
            let num = matches.value_of("number").unwrap().parse::<u32>().unwrap();
            let languages = matches
                .values_of("languages")
                .unwrap()
                .map(str::to_string)
                .collect::<Vec<_>>();
            (num, languages)
        })
}

#[derive(Debug)]
pub enum Hint {
    First(usize),
    Random(usize),
}

pub(crate) fn hint(args: &mut serenity::framework::standard::Args) -> clap::Result<Hint> {
    App::new("hint")
        .version("0.0.1")
        .setting(AppSettings::ColorNever)
        .arg(
            Arg::with_name("number")
                .required(true)
                .validator(parse_validator::<usize>)
                .help("Number of hint characters"),
        )
        .arg(
            Arg::with_name("random")
                .short("r")
                .long("random")
                .takes_value(false)
                .help("Flag for random select hint")
                .required(false),
        )
        .get_matches_from_safe(
            std::iter::once("hint".to_string()).chain(args.iter::<String>().filter_map(Result::ok)),
        )
        .map(|matches| {
            let num = matches
                .value_of("number")
                .unwrap()
                .parse::<usize>()
                .unwrap();
            if matches.is_present("random") {
                Hint::Random(num)
            } else {
                Hint::First(num)
            }
        })
}

pub(crate) fn prefix(
    args: &mut serenity::framework::standard::Args,
) -> clap::Result<Option<String>> {
    App::new("prefix")
        .version("0.0.1")
        .setting(AppSettings::ColorNever)
        .subcommand(
            SubCommand::with_name("set")
                .about("set new prefix")
                .setting(AppSettings::ColorNever)
                .arg(
                    Arg::with_name("prefix")
                        .required(true)
                        .validator(prefix_validator)
                        .help("Sets new prefix if any"),
                ),
        )
        .get_matches_from_safe(
            std::iter::once("prefix".to_string())
                .chain(args.iter::<String>().filter_map(Result::ok)),
        )
        .map(|matches| {
            matches
                .subcommand_matches("set")
                .map(|arg| arg.value_of("prefix").map(str::to_string))
                .flatten()
        })
}
