//#![feature(async_await)]
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate if_chain;
#[macro_use] extern crate custom_derive;
#[macro_use] extern crate enum_derive;
extern crate clap;
extern crate regex;
extern crate toml;
extern crate unicode_segmentation;
//extern crate nazonazo_macros;

use std::env;
use unicode_segmentation::UnicodeSegmentation;
use regex::Regex;
use serenity::{
    client::{
        Client,
    },
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
        StandardFramework,
    },
    model::{
        channel::Message,
        gateway::Ready,
    },
    prelude::*,
};

use std::fs::File;
use std::io::{BufWriter, Write};
use std::iter::FromIterator;
use std::str::from_utf8;
use std::collections::BTreeSet;


pub mod dictionary;

macro_rules! count {
    ( $x:ident ) => (1usize);
    ( $x:ident, $($xs:tt)* ) => (1usize + count!($($xs)*));
}

macro_rules! quiz_commands {
    () => {};
    ( $command:ident, $( $commands:ident ),* ) => {
        group!({
            name: "quiz",
            options: {},
            commands: [$command, $($commands),*],
        });
        const COMMAND_NUM: usize = count!($($commands),*) + 1;
        lazy_static! {
            pub static ref QUIZ_COMMANDS: [String; COMMAND_NUM] = [stringify!($command).to_string(), $(stringify!($commands).to_string(),)*];
            pub static ref QUIZ_COMMANDS_REGEX: Regex = Regex::new(
                &vec!["^(", stringify!($command), $("|", stringify!($commands),)* ")$"].join("")
            ).unwrap();
        }
    };
}

quiz_commands!(en, ja, fr, de, it);
/*
quiz_commands! {
    en: {
        dictionary = english,
    },
    ja: {
        dictionary = japanese,
    },
    fr: {
        dictionary = french,
    },
    de: {
        dictionary = german,
    },
};
*/
group!({
    name: "extra",
    options: {},
    commands: [giveup, hint],
});

group!({
    name: "contest",
    options: {},
    commands: [contest, unrated],
});

group!({
    name: "help",
    options: {},
    commands: [help],
});
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

pub mod command {
    use clap::{App, AppSettings, Arg};

    fn range_validator(low: u32, up: u32) -> Box<dyn Fn(String) -> Result<(), String>> {
        Box::new(move |num: String| match num.parse::<u32>() {
            Err(_) => Err(String::from("please specify unsigned integer after '~contest'.")),
            Ok(num) if num == low => Err(String::from("too small number.")),
            Ok(num) if num > up => Err(String::from("too large number.")),
            Ok(_) => Ok(()),
        })
    }
    fn parse_validator<T: std::str::FromStr>(num: String) -> Result<(), String> {
        num.parse::<T>().map(|_|()).map_err(|_| format!("`{}` is invalid.", num).to_string())
    }
    fn language_validator(language: String) -> Result<(), String> {
        if !crate::QUIZ_COMMANDS_REGEX.is_match(&language) {
            Err(format!("unexpected language '{}'.", language).to_string())
        } else {
            Ok(())
        }
    }
    pub fn contest(args: &mut serenity::framework::standard::Args) -> clap::Result<(u32, Vec<String>)> {
        App::new("contest")
            .version(crate::VERSION)
            .setting(AppSettings::ColorNever)
            .arg(
                Arg::with_name("number")
                    .required(true)
                    .validator(range_validator(1, 100)))
            .arg(
                Arg::with_name("languages")
                    .required(true)
                    .use_delimiter(true)
                    .validator(language_validator)
                    .min_values(1))
            .get_matches_from_safe(
                vec!["contest".to_string()]
                    .into_iter()
                    .chain(args.iter::<String>().filter_map(Result::ok))
                    .into_iter())
            .map(|a| {
                let num = a.value_of("number").unwrap().parse::<u32>().unwrap();
                let languages = a.values_of("languages").unwrap().map(str::to_string).collect::<Vec<_>>();
                (num, languages)
            })
    }
    pub fn hint(args: &mut serenity::framework::standard::Args) -> clap::Result<usize> {
        App::new("hint")
            .version(crate::VERSION)
            .setting(AppSettings::ColorNever)
            .arg(
                Arg::with_name("number")
                    .required(true)
                    .validator(parse_validator::<usize>),
            )
            .get_matches_from_safe(
                vec!["contest".to_string()]
                    .into_iter()
                    .chain(args.iter::<String>().filter_map(Result::ok))
                    .into_iter())
            .map(|a| a.value_of("number").unwrap().parse::<usize>().unwrap())
    }
}

pub mod bot {

    use super::dictionary::*;
    use indexmap::{IndexSet};
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::RwLock;
    use std::sync::{Arc, Mutex};
    use rand::distributions::{Distribution, Uniform};

    custom_derive! {
        #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, NextVariant, PrevVariant)]
        pub enum Lang {
            En,
            Ja,
            Fr,
            De,
            It,
        }
    }

    impl Lang {
        pub fn as_symbol(&self) -> String {
            match self {
                Lang::En => "英単語".to_string(),
                Lang::Ja => "単語".to_string(),
                Lang::Fr => "仏単語".to_string(),
                Lang::De => "独単語".to_string(),
                Lang::It => "伊単語".to_string(),
            }
        }
    }

    impl<S: Into<String>> From<S> for Lang {
        fn from(s: S) -> Self {
            let lang: String = s.into();
            match lang {
                en if en == "en" => Lang::En,
                ja if ja == "ja" => Lang::Ja,
                fr if fr == "fr" => Lang::Fr,
                de if de == "de" => Lang::De,
                it if it == "it" => Lang::It,
                _ => panic!("unexpected language token!"),
            }
        }
    }

    pub enum Status {
        StandingBy,
        Holding(String, String, BTreeSet<String>, Option<BTreeSet<String>>),
        Contesting(String, String, (u32, u32), BTreeSet<String>, Option<BTreeSet<String>>),
    }

    impl Status {
        pub fn is_standing_by(&self) -> bool {
            match self {
                Status::StandingBy => true,
                _ => false,
            }
        }
        pub fn is_holding(&self) -> bool {
            match self {
                Status::Holding(..) => true,
                _ => false,
            }
        }
        pub fn is_contesting(&self) -> bool {
            match self {
                Status::Contesting(..) => true,
                _ => false,
            }
        }

        pub fn ans(&self) -> std::result::Result<&String, ()> {
            match self {
                Status::StandingBy => Err(()),
                Status::Holding(ans, ..) | Status::Contesting(ans, ..) => Ok(ans),
            }
        }
    }

    pub fn select_dictionary(lang: Lang) -> &'static Dictionary {
        match lang {
            Lang::En => &*ENGLISH,
            Lang::Ja => &*JAPANESE,
            Lang::Fr => &*FRENCH,
            Lang::De => &*GERMAN,
            Lang::It => &*ITALIAN,
        }
    }

    pub fn select_dictionary_from_str<S: Into<String>>(lang: S) -> &'static Dictionary {
        let lang_string: String = lang.into();
        match lang_string {
            en if en == "en" => &*ENGLISH,
            ja if ja == "ja" => &*JAPANESE,
            fr if fr == "fr" => &*FRENCH,
            de if de == "de" => &*GERMAN,
            it if it == "it" => &*ITALIAN,
            _ => panic!("unexpected language token!"),
        }
    }

    pub struct DictionarySelector {
        engine: Result<Lang, Uniform<usize>>,
        set: IndexSet<Lang>,
    }

    impl DictionarySelector {
        pub fn new() -> DictionarySelector {
            DictionarySelector {
                engine: Ok(Lang::En),
                set: Default::default()
            }
        }
        pub fn set<S: Into<String>>(&mut self, languages: Vec<S>) {
            if languages.len() == 1 {
                self.engine = Ok(Lang::from(languages.into_iter().next().unwrap()));
            }
            else {
                self.engine = Err(Uniform::new(0, languages.len()));
                for lang in languages {
                    self.set.insert(Lang::from(lang));
                }
            }
        }
        pub fn select<Engine: rand::Rng>(&self, rng: &mut Engine) -> (&'static Dictionary, Lang) {
            let lang =
                *self.engine.as_ref().unwrap_or_else(
                    |uniform| self.set.get_index(uniform.sample(rng)).unwrap()
                );
            (select_dictionary(lang.clone()), lang)
        }
    }

    lazy_static! {
        pub static ref QUIZ: Arc<Mutex<Status>> = Arc::new(Mutex::new(Status::StandingBy));
        pub static ref CONTEST_REUSLT: RwLock<BTreeMap<String, u32>> = RwLock::new(BTreeMap::new());
        pub static ref CONTEST_LIBRARY: RwLock<DictionarySelector> = RwLock::new(DictionarySelector::new());
    }
}

fn prob(ctx: &mut Context, msg: &Message, lang: bot::Lang) -> (String, String, BTreeSet<String>, Option<BTreeSet<String>>) {
    println!("called prob");
    let dic = match lang {
        bot::Lang::En => &*dictionary::ENGLISH,
        bot::Lang::Ja => &*dictionary::JAPANESE,
        bot::Lang::Fr => &*dictionary::FRENCH,
        bot::Lang::De => &*dictionary::GERMAN,
        bot::Lang::It => &*dictionary::ITALIAN,
    };
    let (ans, sorted) = dic.get(&mut rand::thread_rng()).unwrap();
    msg.channel_id
        .say(
            &ctx,
            format!(
                "ソートなぞなぞ ソート前の {as_str} な〜んだ？\n{prob}",
                as_str = lang.as_symbol(),
                prob = sorted
            ),
        )
        .expect("fail to post");
    (ans.clone(), sorted.clone(), dic.get_anagrams(&sorted), dic.get_full_anagrams(&sorted))
}

fn contest_continue(
    ctx: &mut Context,
    msg: &Message,
    count: u32,
    num: u32,
) -> (String, String, BTreeSet<String>, Option<BTreeSet<String>>) {
    let (dic, lang) = bot::CONTEST_LIBRARY.read().unwrap().select(&mut rand::thread_rng());
    let (ans, sorted) = dic.get(&mut rand::thread_rng()).unwrap();
    msg.channel_id
        .say(
            &ctx,
            format!(
                "問 {current} ({current}/{number})\nソートなぞなぞ ソート前の {symbol} な〜んだ？\n{prob}",
                number = num,
                current = count,
                prob = sorted,
                symbol = lang.as_symbol(),
            ),
        )
        .expect("fail to post");
    (ans.clone(), sorted.clone(), dic.get_anagrams(&sorted), dic.get_full_anagrams(&sorted))
}

fn kick(ctx: &mut Context, msg: &Message) -> std::io::Result<()> {
    use std::process::{Command};
    let mut src = BufWriter::new(File::create("/tmp/main.rs")?);
    let code = format!(
        r#"fn kick() {{
    println!("ヒィンｗ");
}}
fn main() {{
    {}
}}
"#,
        &msg.content
    );
    println!("{}", code);
    src.write_all(code.as_bytes())?;
    src.flush()?;
    match Command::new("rustc").arg("/tmp/main.rs").output() {
        Ok(output) => {
            if output.status.success() {
                msg.channel_id.say(&ctx, "ヒィンｗ").expect("fail to post");
            } else {
                msg.channel_id
                    .say(&ctx, from_utf8(output.stderr.as_slice()).unwrap())
                    .expect("fail to post");
            }
        },
        Err(e) => {
            msg.channel_id.say(&ctx, format!("{:?}", e)).expect("fail to post");
        },
    }
    Ok(())
}

fn answer_check(ctx: &mut Context, msg: &Message) {
    if let Ok(mut quiz_guard) = bot::QUIZ.lock() {
        match &mut *quiz_guard {
            bot::Status::Holding(ans, _, anagram, full_anagram) => {
                if &ans.to_lowercase() == &msg.content.to_lowercase() {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、正解です！\n正解は\"{}\"でした！",
                                &msg.author.name, &ans
                            ),
                        )
                        .expect("fail to post");
                    *quiz_guard = bot::Status::StandingBy;
                } else if anagram.contains(&msg.content.to_lowercase()) {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、{} は非想定解ですが正解です！",
                                &msg.author.name, &msg.content.to_lowercase()
                            ),
                        )
                        .expect("fail to post");
                    *quiz_guard = bot::Status::StandingBy;
                } else if full_anagram.as_ref().map(|x|x.contains(&msg.content.to_lowercase())).unwrap_or_default() {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、{} は出題辞書に非想定解ですが正解です！",
                                &msg.author.name, &msg.content.to_lowercase()
                            ),
                        )
                        .expect("fail to post");
                    *quiz_guard = bot::Status::StandingBy;
                }
            },
            bot::Status::Contesting(ref ans, _, (count, num), anagram, full_anagram) => {
                let mut finally = false;
                if &ans.to_lowercase() == &msg.content.to_lowercase() {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、正解です！\n正解は\"{}\"でした！",
                                &msg.author.name, &ans
                            ),
                        )
                        .expect("fail to post");
                    finally = true;
                } else if anagram.contains(&msg.content.to_lowercase()) {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、{} は非想定解ですが正解です！",
                                &msg.author.name, &msg.content.to_lowercase()
                            ),
                        )
                        .expect("fail to post");
                    finally = true;
                } else if full_anagram.as_ref().map(|x|x.contains(&msg.content.to_lowercase())).unwrap_or_default() {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、{} は出題辞書に非想定解ですが正解です！",
                                &msg.author.name, &msg.content.to_lowercase()
                            ),
                        )
                        .expect("fail to post");
                    finally = true;
                }

                if_chain! {
                    if finally;
                    let _ = *bot::CONTEST_REUSLT
                        .write()
                        .unwrap()
                        .entry(msg.author.name.clone())
                        .or_insert(0) += 1;
                    if count >= num;
                    if let Ok(guard) = bot::CONTEST_REUSLT.read();
                    then {
                        let mut v = Vec::from_iter(guard.iter());
                        v.sort_by(|&(_, a), &(_, b)| b.cmp(&a));
                        msg.channel_id
                            .say(
                                &ctx,
                                format!(
                                    "{num}問連続のコンテストが終了しました。\n{result}",
                                    num = num,
                                    result = v
                                        .into_iter()
                                        .map(|tuple| format!(
                                            "{} AC: {}\n",
                                            tuple.1, tuple.0
                                        ))
                                        .collect::<String>()
                                ),
                            )
                            .expect("fail to post");
                        *bot::CONTEST_REUSLT.write().unwrap() = std::collections::BTreeMap::new();
                        *quiz_guard = bot::Status::StandingBy;
                    } else {
                        let (ans, sorted, anagram, full_anagram) =
                            contest_continue(ctx, &msg, *count + 1, *num);
                        *quiz_guard = bot::Status::Contesting(
                            ans.clone(),
                            sorted.clone(),
                            (*count + 1, *num),
                            anagram,
                            full_anagram,
                        );
                    }
                }
            },
            _ => {}
        }
    }
}

fn main() {
    // Login with a bot token from the environment
    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("token"), Handler)
        .expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
            .before(|ctx, msg, command_name| {
                println!(
                    "Got command '{}' by user '{}'",
                    command_name, msg.author.name
                );
                if QUIZ_COMMANDS.contains(&command_name.to_string()) {
                    match &*bot::QUIZ.lock().unwrap() {
                        bot::Status::Holding(_, ref sorted, ..) => {
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!("前回の出題が解かれていません\n問題: {}", sorted),
                                )
                                .expect("fail to post");
                            false
                        }
                        bot::Status::Contesting(_, ref sorted, ..) => {
                            msg.channel_id
                                .say(&ctx, format!("現在コンテスト中です\n問題: {}", sorted))
                                .expect("fail to post");
                            false
                        }
                        bot::Status::StandingBy => true,
                    }
                } else {
                    true
                }
            })
            .normal_message(|ctx, msg| {
                let re = Regex::new(r"^kick\(.*\);$").unwrap();
                if re.is_match(&msg.content) {
                    println!("{:?}", kick(ctx, msg));
                    return;
                }
                println!("got message \"{}\"", &msg.content);
                answer_check(ctx, msg);
            })
            .group(&QUIZ_GROUP)
            .group(&CONTEST_GROUP)
            .group(&HELP_GROUP)
            .group(&EXTRA_GROUP),
    );

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
fn en(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~en' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = prob(ctx, &msg, bot::Lang::En);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), anagram, full_anagram);
        }
    }
    Ok(())
}

#[command]
fn ja(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~ja' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = prob(ctx, &msg, bot::Lang::Ja);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), anagram, full_anagram);
        }
    }
    Ok(())
}
#[command]
fn fr(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~fr' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = prob(ctx, &msg, bot::Lang::Fr);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), anagram, full_anagram);
        }
    }
    Ok(())
}
#[command]
fn de(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~de' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = prob(ctx, &msg, bot::Lang::De);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), anagram, full_anagram);
        }
    }
    Ok(())
}
#[command]
fn it(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~it' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = prob(ctx, &msg, bot::Lang::It);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), anagram, full_anagram);
        }
    }
    Ok(())
}

fn giveup_impl(
    ctx: &mut Context,
    msg: &Message,
) -> CommandResult {
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            match &mut *guard {
                bot::Status::Holding(ans, ..) => {
                    msg.channel_id
                        .say(&ctx, format!("正解は \"{}\" でした...", ans))
                        .expect("fail to post");
                    *guard = bot::Status::StandingBy;
                },
                bot::Status::Contesting(ans, _, (count, num), ..) => {
                    msg.channel_id
                        .say(&ctx, format!("正解は \"{}\" でした...", ans))
                        .expect("fail to post");
                    if &count == &num {
                        msg.channel_id
                            .say(&ctx, format!("{}問連続のコンテストが終了しました。", num))
                            .expect("fail to post");
                        *guard = bot::Status::StandingBy;
                    }
                },
                bot::Status::StandingBy => {
                    msg.channel_id
                        .say(&ctx, "現在問題は出ていません。")
                        .expect("fail to post");
                },
            }
        }
    }
    Ok(())
}

#[command]
fn giveup(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~giveup' by user '{}'", msg.author.name);
    giveup_impl(ctx, msg)
}

#[command]
fn contest(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    use crate::bot::CONTEST_LIBRARY;
    println!("Got command '~contest' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut quiz_guard) = bot::QUIZ.lock();
        then {
            match command::contest(&mut args) {
                Err(err_msg) => {
                    msg.channel_id.say(&ctx, err_msg).expect("fail to post");
                    return Ok(());
                }
                Ok((num, mut languages)) => {
                    languages.sort();
                    languages.dedup();
                    CONTEST_LIBRARY.write().unwrap().set(languages);
                    let (dic, lang) = CONTEST_LIBRARY.write().unwrap().select(&mut rand::thread_rng());
                    let (ans, sorted) = dic.get(&mut rand::thread_rng()).unwrap();
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{number}問のコンテストを始めます。\n問 1 (1/{number})\nソートなぞなぞ ソート前の {symbol} な〜んだ？\n{prob}",
                                number = num,
                                prob = sorted,
                                symbol = lang.as_symbol(),
                            ),
                        )
                        .expect("fail to post");
                    *quiz_guard = bot::Status::Contesting(ans.clone(), sorted.clone(), (1, num), dic.get_anagrams(&sorted), dic.get_full_anagrams(&sorted));
                }
            }
        }
    }
    Ok(())
}

#[command]
fn unrated(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~unrated' by user '{}'", msg.author.name);
    loop {
        if let Ok(mut guard) = bot::QUIZ.lock() {
            if guard.is_contesting() {
                msg.channel_id
                    .say(&ctx, "コンテストを中止します。")
                    .expect("fail to post");
                *guard = bot::Status::StandingBy;
                *bot::CONTEST_REUSLT.write().unwrap() = std::collections::BTreeMap::new();
            } else {
                msg.channel_id
                    .say(&ctx, "現在コンテストは開催されていません。")
                    .expect("fail to post");
            }
            break;
        }
    }
    Ok(())
}

#[command]
fn hint(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    println!("Got command '~hint' by user '{}'", msg.author.name);

    if_chain! {
        if !msg.author.bot;
        if let Ok(guard) = bot::QUIZ.lock();
        if guard.is_holding() || guard.is_contesting();
        then {
            let mut g = UnicodeSegmentation::graphemes(guard.ans().unwrap().as_str(), true).collect::<Vec<&str>>();
            match command::hint(&mut args) {
                Err(err_msg) => {
                    msg.channel_id
                        .say(&ctx, format!("{}", err_msg))
                        .expect("fail to post");
                },
                Ok(num) if num == 0 => {
                    msg.channel_id
                        .say(&ctx, "ゼロ文字ヒントは意味ねえよ、ボケ！")
                        .expect("fail to post");
                },
                Ok(num) if num == g.len() || num == g.len() - 1 => {
                    drop(guard);
                    msg.channel_id
                        .say(&ctx, "答えが一意に定まるためギブアップとみなされました！")
                        .expect("fail to post");
                    giveup_impl(ctx, msg)?;
                },
                Ok(num) if num > g.len() => {
                    msg.channel_id
                        .say(&ctx, "問題の文字数より長えよボケが！")
                        .expect("fail to post");
                },
                Ok(num) => {
                    g.truncate(num);
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "答えの先頭 {len} 文字は... => \"{hint}\" ",
                                len = num,
                                hint = g.into_iter().collect::<String>(),
                            ),
                        )
                        .expect("fail to post");
                },
            }
        }
    }
    Ok(())
}

#[command]
fn help(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~help' by user '{}'", msg.author.name);

    if !msg.author.bot {
        msg.channel_id
            .say(
                &ctx,
                format!(
                    r#"
sort_nazonazo v{version}
mitama <yussa.de.dannann@gmail.com>

USAGE [QUIZ]:
    ~{{LANG}}: LANG=[en|ja|fr|de|it]
    => その言語で単体のクイズが出ます

    ~giveup:
    => 一問を諦めて答えを見ることができます（出題状態はキャンセルされます）。

    ~hint {{NUM>=answer length}}:
    => 答えの最初のNUM文字をヒントとして見ることができます。

USAGE [CONTEST]:
    ~contest {{LANG}} {{NUM<=100}}: LANG=[en|ja|fr|de|it]
    => 言語オンリー連続出題を行います

    ~unrated:
    => コンテストを中止します

USEGE [EXTRA]:
    ~help:
    => 今あなたが使ったコマンドです。見てのとおりです。
                "#,
                    version = VERSION
                ),
            )
            .expect("fail to post");
    }
    Ok(())
}
