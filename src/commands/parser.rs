use clap::{App, AppSettings, Arg};
use super::facade;

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
        .map_err(|_| format!("`{}` is invalid.", num).to_string())
}

fn language_validator(language: String) -> Result<(), String> {
    if !facade::QUIZ_COMMANDS_REGEX.is_match(&language) {
        Err(format!("unexpected language '{}'.", language).to_string())
    } else {
        Ok(())
    }
}

pub(crate)  fn contest(
    args: &mut serenity::framework::standard::Args,
) -> clap::Result<(u32, Vec<String>)> {
    App::new("contest")
        .version("0.0.1")
        .setting(AppSettings::ColorNever)
        .arg(
            Arg::with_name("number")
                .required(true)
                .validator(range_validator(1, 100)),
        )
        .arg(
            Arg::with_name("languages")
                .required(true)
                .use_delimiter(true)
                .validator(language_validator)
                .min_values(1),
        )
        .get_matches_from_safe(
            vec!["contest".to_string()]
                .into_iter()
                .chain(args.iter::<String>().filter_map(Result::ok))
                .into_iter(),
        )
        .map(|a| {
            let num = a.value_of("number").unwrap().parse::<u32>().unwrap();
            let languages = a
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
                .help("flag for random select hint")
                .required(false)
        )
        .get_matches_from_safe(
            vec!["contest".to_string()]
                .into_iter()
                .chain(args.iter::<String>().filter_map(Result::ok))
                .into_iter(),
        )
        .map(|matches|{
            let num = matches.value_of("number").unwrap().parse::<usize>().unwrap();
            if dbg!(matches.is_present("random")) {
                Hint::Random(num)
            } else {
                Hint::First(num)
            }
        })
}
