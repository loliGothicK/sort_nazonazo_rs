//#![feature(async_await)]
use rand::distributions::{Distribution, Uniform};
use std::env;
#[macro_use]
extern crate lazy_static;
extern crate clap;
extern crate regex;
extern crate serde_derive;
extern crate serde_json;
extern crate toml;
extern crate unicode_segmentation;
#[macro_use]
extern crate paste;
// A trait that the Validate derive will impl
use unicode_segmentation::UnicodeSegmentation;
//extern crate nazonazo_macros;
use clap::{App, Arg, SubCommand};
use regex::Regex;
#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;

use serenity::{
    client::{
        bridge::gateway::{ShardId, ShardManager},
        Client,
    },
    framework::standard::{
        help_commands,
        macros::{check, command, group, help},
        Args, CheckResult, CommandGroup, CommandOptions, CommandResult, DispatchError, HelpOptions,
        StandardFramework,
    },
    model::{
        channel::{Channel, Message},
        gateway::Ready,
        id::UserId,
    },
    prelude::*,
    utils::{content_safe, ContentSafeOptions, MessageBuilder},
};

use indexmap::IndexSet;
use itertools::Itertools;
use regex::internal::Input;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::iter::FromIterator;
use std::str::from_utf8;
use tokio::future::err;

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
        const COMMAND_NUM: usize = count!($($commands),*);
        lazy_static! {
            pub static ref QUIZ_COMMANDS: [String; COMMAND_NUM] = [$(stringify!($commands).to_string(),)*];
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
    use regex::Regex;

    fn number_validator(num: String) -> Result<(), String> {
        match num.parse::<u32>() {
            Err(_) => Err(String::from("please specify number after '~contest'.")),
            Ok(num) if num == 0 => Err(String::from("invalid number.")),
            Ok(num) if num > 100 => Err(String::from("too large number.")),
            Ok(_) => Ok(()),
        }
    }

    fn language_validator(language: String) -> Result<(), String> {
        //        let re = Regex::new("^(en|ja|fr|de|it)$").unwrap();
        if !crate::QUIZ_COMMANDS_REGEX.is_match(&language) {
            Err(format!("unexpected language '{}'.", language).to_string())
        } else {
            Ok(())
        }
    }

    pub fn contest() -> App<'static, 'static> {
        App::new("contest")
            .version("v1.0-beta")
            .setting(AppSettings::ColorNever)
            .arg(
                Arg::with_name("number")
                    .required(true)
                    .validator(number_validator),
            )
            .arg(
                Arg::with_name("languages")
                    .required(true)
                    .use_delimiter(true)
                    .validator(language_validator)
                    .min_values(1),
            )
    }
}

pub mod bot {

    use super::dictionary::*;
    use indexmap::{IndexMap, IndexSet};
    use rand::distributions::{Distribution, Uniform};
    use std::collections::BTreeMap;
    use std::sync::RwLock;
    use std::sync::{Arc, Mutex};

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
        pub fn as_string(&self) -> String {
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
        Holding(String, String, Lang),
        Contesting(String, String, Lang, (u32, u32)),
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
                Status::Holding(ans, ..) => Ok(ans),
                Status::Contesting(ans, ..) => Ok(ans),
            }
        }
    }

    pub fn select_dictionary(lang: &Lang) -> &Dictionary {
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

    lazy_static! {
        pub static ref QUIZ: Arc<Mutex<Status>> = Arc::new(Mutex::new(Status::StandingBy));
        pub static ref CONTEST_REUSLT: RwLock<BTreeMap<String, u32>> = RwLock::new(BTreeMap::new());
        pub static ref CONTEST_LANGUAGES: Arc<Mutex<IndexSet<Lang>>> =
            Arc::new(Mutex::new(IndexSet::new()));
        pub static ref DISTRIBUTION: RwLock<Uniform<usize>> = RwLock::new(Uniform::new(0, 1));
    }
}

fn prob(ctx: &mut Context, msg: &Message, lang: bot::Lang) -> (String, String) {
    println!("called prob");
    let dic = match lang {
        bot::Lang::En => &*dictionary::ENGLISH,
        bot::Lang::Ja => &*dictionary::JAPANESE,
        bot::Lang::Fr => &*dictionary::FRENCH,
        bot::Lang::De => &*dictionary::GERMAN,
        bot::Lang::It => &*dictionary::ITALIAN,
    };
    println!("len = {}", dic.len());
    let (ans, sorted) = dic.get(&mut rand::thread_rng()).unwrap();
    msg.channel_id
        .say(
            &ctx,
            format!(
                "ソートなぞなぞ ソート前の {as_str} な〜んだ？\n{prob}",
                as_str = lang.as_string(),
                prob = sorted
            ),
        )
        .expect("fail to post");
    (ans.clone(), sorted.clone())
}

fn contest_continue(
    ctx: &mut Context,
    msg: &Message,
    lang: bot::Lang,
    count: u32,
    num: u32,
) -> (String, String) {
    let dic = match lang {
        bot::Lang::En => &*dictionary::ENGLISH,
        bot::Lang::Ja => &*dictionary::JAPANESE,
        bot::Lang::Fr => &*dictionary::FRENCH,
        bot::Lang::De => &*dictionary::GERMAN,
        bot::Lang::It => &*dictionary::ITALIAN,
    };
    if bot::CONTEST_LANGUAGES.lock().unwrap().len() == 1 {
        let (ans, sorted) = dic.get(&mut rand::thread_rng()).unwrap();
        msg.channel_id
            .say(
                &ctx,
                format!(
                    "問 {current} ({current}/{number})\nソートなぞなぞ ソート前の {lang} な〜んだ？\n{prob}",
                    number = num,
                    current = count,
                    prob = sorted,
                    lang = lang.as_string(),
                ),
            )
            .expect("fail to post");
        (ans.clone(), sorted.clone())
    } else {
        let langs = bot::CONTEST_LANGUAGES.lock().unwrap();
        let lang = langs
            .get_index(
                bot::DISTRIBUTION
                    .write()
                    .unwrap()
                    .sample(&mut rand::thread_rng()),
            )
            .unwrap();
        let dic = bot::select_dictionary(&lang);
        let (ans, sorted) = dic.get(&mut rand::thread_rng()).unwrap();
        msg.channel_id
            .say(
                &ctx,
                format!(
                    "問 {current} ({current}/{number})\nソートなぞなぞ ソート前の {lang} な〜んだ？\n{prob}",
                    number = num,
                    current = count,
                    prob = sorted,
                    lang = lang.as_string(),
                ),
            )
            .expect("fail to post");
        (ans.clone(), sorted.clone())
    }
}

fn kick(ctx: &mut Context, msg: &Message) -> std::io::Result<()> {
    use std::process::{Command, ExitStatus, Output};
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
                msg.channel_id.say(&ctx, "ヒィンｗ");
            } else {
                msg.channel_id
                    .say(&ctx, from_utf8(output.stderr.as_slice()).unwrap());
            }
        }
        Err(e) => {
            msg.channel_id.say(&ctx, format!("{:?}", e));
        }
    }
    Ok(())
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
                        bot::Status::Holding(ref sorted, ..) => {
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!("前回の出題が解かれていません\n問題: {}", sorted),
                                )
                                .expect("fail to post");
                            false
                        }
                        bot::Status::Contesting(ref sorted, ..) => {
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
                if let Ok(mut guard) = bot::QUIZ.lock() {
                    match &mut *guard {
                        bot::Status::Holding(ans, _sorted, _lang)
                            if &ans.to_lowercase() == &msg.content.to_lowercase() =>
                        {
                            println!("pass Ok arm: ans = {}.", ans);
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!(
                                        "{} さん、正解です！\n正解は\"{}\"でした！",
                                        &msg.author.name, &ans
                                    ),
                                )
                                .expect("fail to post");
                            *guard = bot::Status::StandingBy;
                        }
                        bot::Status::Contesting(ref ans, _, lang, (count, num))
                            if &ans.to_lowercase() == &msg.content.to_lowercase() =>
                        {
                            println!("pass Ok arm: ans = {}.", ans);
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!(
                                        "{} さん、正解です！\n正解は\"{}\"でした！",
                                        &msg.author.name, &ans
                                    ),
                                )
                                .expect("fail to post");
                            *bot::CONTEST_REUSLT
                                .write()
                                .unwrap()
                                .entry(msg.author.name.clone())
                                .or_insert(0) += 1;
                            if count >= num {
                                if let Ok(guard) = bot::CONTEST_REUSLT.read() {
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
                                }
                                *bot::CONTEST_REUSLT.write().unwrap() =
                                    std::collections::BTreeMap::new();
                                *guard = bot::Status::StandingBy;
                                *bot::CONTEST_LANGUAGES.lock().unwrap() = IndexSet::new();
                            } else {
                                let (ans, sorted) =
                                    contest_continue(ctx, &msg, *lang, *count + 1, *num);
                                *guard = bot::Status::Contesting(
                                    ans.clone(),
                                    sorted.clone(),
                                    *lang,
                                    (*count + 1, *num),
                                );
                            }
                        }
                        _ => {}
                    }
                }
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
    if !msg.author.bot {
        if let Ok(mut guard) = bot::QUIZ.lock() {
            let (ans, sorted) = prob(ctx, &msg, bot::Lang::En);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::En);
        }
    };
    Ok(())
}

#[command]
fn ja(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~ja' by user '{}'", msg.author.name);
    if !msg.author.bot {
        if let Ok(mut guard) = bot::QUIZ.lock() {
            let (ans, sorted) = prob(ctx, &msg, bot::Lang::Ja);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::Ja);
        }
    };
    Ok(())
}
#[command]
fn fr(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~fr' by user '{}'", msg.author.name);
    if !msg.author.bot {
        if let Ok(mut guard) = bot::QUIZ.lock() {
            let (ans, sorted) = prob(ctx, &msg, bot::Lang::Fr);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::Fr);
        }
    };
    Ok(())
}
#[command]
fn de(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~de' by user '{}'", msg.author.name);
    if !msg.author.bot {
        if let Ok(mut guard) = bot::QUIZ.lock() {
            let (ans, sorted) = prob(ctx, &msg, bot::Lang::De);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::De);
        }
    };
    Ok(())
}
#[command]
fn it(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~it' by user '{}'", msg.author.name);
    if !msg.author.bot {
        let (ans, sorted) = prob(ctx, &msg, bot::Lang::It);
        if let Ok(mut guard) = bot::QUIZ.lock() {
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::It);
        }
    };
    Ok(())
}

fn giveup_impl(
    ctx: &mut Context,
    msg: &Message,
    quiz: &bot::Status,
) -> Option<(bot::Lang, (u32, u32))> {
    match quiz {
        bot::Status::Holding(ans, _, _) => {
            msg.channel_id
                .say(&ctx, format!("正解は \"{}\" でした...", ans))
                .expect("fail to post");
            None
        }
        bot::Status::Contesting(ans, _, lang, (count, num)) => {
            msg.channel_id
                .say(&ctx, format!("正解は \"{}\" でした...", ans))
                .expect("fail to post");
            Some((*lang, (count + 1, *num)))
        }
        bot::Status::StandingBy => {
            msg.channel_id
                .say(&ctx, "現在問題は出ていません。")
                .expect("fail to post");
            None
        }
    }
}

#[command]
fn giveup(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~giveup' by user '{}'", msg.author.name);
    if !msg.author.bot {
        if let Ok(mut guard) = bot::QUIZ.lock() {
            match giveup_impl(ctx, msg, &*guard) {
                Some((_, (count, num))) if count > num => {
                    msg.channel_id
                        .say(&ctx, format!("{}問連続のコンテストが終了しました。", num))
                        .expect("fail to post");
                    *guard = bot::Status::StandingBy;
                    return Ok(());
                }
                Some((lang, (count, num))) => {
                    let (ans, sorted) = contest_continue(ctx, &msg, lang, count, num);
                    *guard =
                        bot::Status::Contesting(ans.clone(), sorted.clone(), lang, (count, num));
                }
                None => {
                    *guard = bot::Status::StandingBy;
                }
            }
        }
    }
    Ok(())
}

#[command]
fn contest(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    use crate::bot::DISTRIBUTION;
    println!("Got command '~contest' by user '{}'", msg.author.name);
    if !msg.author.bot {
        if let (Ok(mut quiz_guard), Ok(mut contest_languages_guard)) =
            (bot::QUIZ.lock(), bot::CONTEST_LANGUAGES.lock())
        {
            let num = match command::contest().get_matches_from_safe(
                vec!["contest".to_string()]
                    .into_iter()
                    .chain(args.iter::<String>().filter_map(Result::ok))
                    .into_iter(),
            ) {
                Err(err_msg) => {
                    msg.channel_id.say(&ctx, err_msg).expect("fail to post");
                    return Ok(());
                }
                Ok(matches) => {
                    for lang in matches.values_of("languages").unwrap().collect::<Vec<_>>() {
                        let _ = (&mut *contest_languages_guard).insert(bot::Lang::from(lang));
                    }
                    matches.value_of("number").unwrap().parse::<u32>().unwrap()
                }
            };
            *DISTRIBUTION.write().unwrap() = Uniform::new(0, contest_languages_guard.len());
            let lang = (&*contest_languages_guard)
                .get_index(
                    DISTRIBUTION
                        .write()
                        .unwrap()
                        .sample(&mut rand::thread_rng()),
                )
                .unwrap();
            let dic = bot::select_dictionary(&lang);
            let (ans, sorted) = dic.get(&mut rand::thread_rng()).unwrap();
            msg.channel_id
                .say(
                    &ctx,
                    format!(
                        "{number}問のコンテストを始めます。\n問 1 (1/{number})\nソートなぞなぞ ソート前の {lang} な〜んだ？\n{prob}",
                        number = num,
                        prob = sorted,
                        lang = lang.as_string(),
                    ),
                )
                .expect("fail to post");
            *quiz_guard = bot::Status::Contesting(ans.clone(), sorted.clone(), *lang, (1, num));
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
    if !msg.author.bot {
        match args.single::<usize>() {
            Ok(len) => {
                if let Ok(mut guard) = bot::QUIZ.lock() {
                    if guard.is_holding() || guard.is_contesting() {
                        let mut g =
                            UnicodeSegmentation::graphemes(guard.ans().unwrap().as_str(), true)
                                .collect::<Vec<&str>>();
                        match len {
                            length if length < g.len() && length >= g.len() - 1 => {
                                msg.channel_id
                                    .say(
                                        &ctx,
                                        "このヒントで答えが一意に定まるのでgiveupとみなします！",
                                    )
                                    .expect("fail to post");
                                match giveup_impl(ctx, msg, &*guard) {
                                    Some((_, (count, num))) if count > num => {
                                        msg.channel_id
                                            .say(
                                                &ctx,
                                                format!(
                                                    "{}問連続のコンテストが終了しました。",
                                                    num
                                                ),
                                            )
                                            .expect("fail to post");
                                        *guard = bot::Status::StandingBy;
                                        return Ok(());
                                    }
                                    Some((lang, (count, num))) => {
                                        let (ans, sorted) =
                                            contest_continue(ctx, &msg, lang, count, num);
                                        *guard = bot::Status::Contesting(
                                            ans.clone(),
                                            sorted.clone(),
                                            lang,
                                            (count, num),
                                        );
                                    }
                                    None => {
                                        *guard = bot::Status::StandingBy;
                                    }
                                }
                            }
                            length if length == 0usize => {
                                msg.channel_id
                                    .say(&ctx, "0文字のヒントは出せません！")
                                    .expect("fail to post");
                            }
                            length if length < g.len() - 1 => {
                                g.truncate(len);
                                msg.channel_id
                                    .say(
                                        &ctx,
                                        format!(
                                            "答えの先頭 {len} 文字は... => \"{hint}\" ",
                                            len = len,
                                            hint = g.into_iter().collect::<String>(),
                                        ),
                                    )
                                    .expect("fail to post");
                            }
                            _ => {
                                msg.channel_id
                                    .say(&ctx, "ヒントが単語より長過いわ、ボケ")
                                    .expect("fail to post");
                            }
                        }
                    } else {
                        msg.channel_id
                            .say(&ctx, "現在問題は出ていません。")
                            .expect("fail to post");
                    }
                }
            }
            Err(e) => {
                msg.channel_id
                    .say(&ctx, format!("{:?}", e))
                    .expect("fail to post");
            }
        }
    };

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
