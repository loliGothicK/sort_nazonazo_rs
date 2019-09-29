//#![feature(async_await)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate if_chain;
#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;
extern crate clap;
extern crate regex;
extern crate toml;
extern crate unicode_segmentation;
//extern crate nazonazo_macros;

use regex::Regex;
use serenity::{
    client::Client,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult, StandardFramework,
    },
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::env;
use unicode_segmentation::UnicodeSegmentation;

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::iter::FromIterator;
use std::str::from_utf8;

pub mod dictionary;
pub mod command;
pub mod bot;

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

fn prob(
    ctx: &mut Context,
    msg: &Message,
    lang: bot::Lang,
) -> (String, String, BTreeSet<String>, Option<BTreeSet<String>>) {
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
    (
        ans.clone(),
        sorted.clone(),
        dic.get_anagrams(&sorted),
        dic.get_full_anagrams(&sorted),
    )
}

fn contest_continue(
    ctx: &mut Context,
    msg: &Message,
    count: u32,
    num: u32,
) -> (String, String, BTreeSet<String>, Option<BTreeSet<String>>) {
    let (dic, lang) = bot::CONTEST_LIBRARY
        .read()
        .unwrap()
        .select(&mut rand::thread_rng());
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
    (
        ans.clone(),
        sorted.clone(),
        dic.get_anagrams(&sorted),
        dic.get_full_anagrams(&sorted),
    )
}

fn kick(ctx: &mut Context, msg: &Message) -> std::io::Result<()> {
    use std::process::Command;
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
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx, format!("{:?}", e))
                .expect("fail to post");
        }
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
                                &msg.author.name,
                                &msg.content.to_lowercase()
                            ),
                        )
                        .expect("fail to post");
                    *quiz_guard = bot::Status::StandingBy;
                } else if full_anagram
                    .as_ref()
                    .map(|x| x.contains(&msg.content.to_lowercase()))
                    .unwrap_or_default()
                {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、{} は出題辞書に非想定解ですが正解です！",
                                &msg.author.name,
                                &msg.content.to_lowercase()
                            ),
                        )
                        .expect("fail to post");
                    *quiz_guard = bot::Status::StandingBy;
                }
            }
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
                                &msg.author.name,
                                &msg.content.to_lowercase()
                            ),
                        )
                        .expect("fail to post");
                    finally = true;
                } else if full_anagram
                    .as_ref()
                    .map(|x| x.contains(&msg.content.to_lowercase()))
                    .unwrap_or(false)
                {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、{} は出題辞書に非想定解ですが正解です！",
                                &msg.author.name,
                                &msg.content.to_lowercase()
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
                    if let Ok(guard) = bot::CONTEST_REUSLT.read();
                    then {
                        if count >= num {
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
                }
            }
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

fn giveup_impl(ctx: &mut Context, msg: &Message, quiz_stat: &mut bot::Status) -> CommandResult {
    if !msg.author.bot {
        match quiz_stat {
            bot::Status::Holding(ans, ..) => {
                msg.channel_id
                    .say(&ctx, format!("正解は \"{}\" でした...", ans))
                    .expect("fail to post");
                *quiz_stat = bot::Status::StandingBy;
            },
            bot::Status::Contesting(ans, _, (count, num), ..) => {
                msg.channel_id
                    .say(&ctx, format!("正解は \"{}\" でした...", ans))
                    .expect("fail to post");
                if let Ok(mut contest_result) = bot::CONTEST_REUSLT.write() {
                    if &count == &num {
                        let mut v = Vec::from_iter(contest_result.iter());
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
                        *contest_result = std::collections::BTreeMap::new();
                        *quiz_stat = bot::Status::StandingBy;
                    } else {
                        let (ans, sorted, anagram, full_anagram) =
                            contest_continue(ctx, &msg, *count + 1, *num);
                        *quiz_stat = bot::Status::Contesting(
                            ans.clone(),
                            sorted.clone(),
                            (*count + 1, *num),
                            anagram,
                            full_anagram,
                        );
                    }
                }
            },
            bot::Status::StandingBy => {
                msg.channel_id
                    .say(&ctx, "現在問題は出ていません。")
                    .expect("fail to post");
            },
        }
    }
    Ok(())
}

#[command]
fn giveup(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~giveup' by user '{}'", msg.author.name);
    if let Ok(mut guard) = bot::QUIZ.lock() {
        giveup_impl(ctx, msg, &mut *guard)?;
    }
    Ok(())
}

#[command]
fn contest(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    use crate::bot::CONTEST_LIBRARY;
    println!("Got command '~contest' by user '{}'", msg.author.name);
    if !msg.author.bot {
        if let Ok(mut quiz_guard) = bot::QUIZ.lock() {
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
        if let Ok(mut guard) = bot::QUIZ.lock();
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
                    msg.channel_id
                        .say(&ctx, "答えが一意に定まるためギブアップとみなされました！")
                        .expect("fail to post");
                    giveup_impl(ctx, msg, &mut *guard)?;
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
