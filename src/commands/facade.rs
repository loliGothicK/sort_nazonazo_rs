use regex::Regex;
use serenity::{
    client::Client,
    framework::standard::{
        macros::{command, group},
        Args, Delimiter, CommandResult, StandardFramework,
    },
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::env;
use unicode_segmentation::UnicodeSegmentation;
use rand::seq::index::sample;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::iter::FromIterator;
use std::str::from_utf8;

use super::super::{bot, dictionary};
use super::{executors, parser};

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

#[command]
pub fn en(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~en' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = executors::prob(ctx, &msg, bot::Lang::En);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), anagram, full_anagram);
        }
    }
    Ok(())
}

#[command]
pub fn ja(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~ja' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = executors::prob(ctx, &msg, bot::Lang::Ja);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), anagram, full_anagram);
        }
    }
    Ok(())
}
#[command]
pub fn fr(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~fr' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = executors::prob(ctx, &msg, bot::Lang::Fr);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), anagram, full_anagram);
        }
    }
    Ok(())
}
#[command]
pub fn de(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~de' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = executors::prob(ctx, &msg, bot::Lang::De);
            *guard = bot::Status::Holding(ans.clone(), sorted.clone(), anagram, full_anagram);
        }
    }
    Ok(())
}
#[command]
pub fn it(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~it' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let (ans, sorted, anagram, full_anagram) = executors::prob(ctx, &msg, bot::Lang::It);
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
                            executors::contest_continue(ctx, &msg, *count + 1, *num);
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
pub fn giveup(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~giveup' by user '{}'", msg.author.name);
    if let Ok(mut guard) = bot::QUIZ.lock() {
        giveup_impl(ctx, msg, &mut *guard)?;
    }
    Ok(())
}

#[command]
pub fn contest(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    use crate::bot::CONTEST_LIBRARY;
    println!("Got command '~contest' by user '{}'", msg.author.name);
    if !msg.author.bot {
        if let Ok(mut quiz_guard) = bot::QUIZ.lock() {
            match parser::contest(&mut args) {
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
pub fn unrated(ctx: &mut Context, msg: &Message) -> CommandResult {
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
pub fn hint(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    println!("Got command '~hint' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        if !guard.is_standing_by();
        then {
            let mut g = UnicodeSegmentation::graphemes(guard.ans().unwrap().as_str(), true).collect::<Vec<&str>>();
            match parser::hint(&mut args) {
                Err(err_msg) => {
                    msg.channel_id
                        .say(&ctx, format!("{}", err_msg))
                        .expect("fail to post");
                },
                Ok(parser::Hint::First(num)) | Ok(parser::Hint::Random(num)) if num == 0 => {
                    msg.channel_id
                        .say(&ctx, "ゼロ文字ヒントは意味ねえよ、ボケ！")
                        .expect("fail to post");
                },
                Ok(parser::Hint::First(num)) | Ok(parser::Hint::Random(num)) if num == g.len() || num == g.len() - 1 => {
                    msg.channel_id
                        .say(&ctx, "答えが一意に定まるためギブアップとみなされました！")
                        .expect("fail to post");
                    giveup_impl(ctx, msg, &mut *guard)?;
                },
                Ok(parser::Hint::First(num)) | Ok(parser::Hint::Random(num)) if num > g.len() => {
                    msg.channel_id
                        .say(&ctx, "問題の文字数より長えよボケが！")
                        .expect("fail to post");
                },
                Ok(parser::Hint::First(num)) => {
                    g.truncate(num);
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "答えの先頭 {len} 文字は... => `{hint}` ",
                                len = num,
                                hint = g.into_iter().collect::<String>(),
                            ),
                        )
                        .expect("fail to post");
                },
                Ok(parser::Hint::Random(num)) => {
                    let star = "*";
                    let mut hit_str: Vec<&str> = std::iter::repeat(star).take(g.len()).collect();
                    for idx in rand::seq::index::sample(&mut rand::thread_rng(), g.len(), num).into_iter() {
                        if let Some(elem) = hit_str.get_mut(idx) {
                            *elem = g.get(idx).unwrap();
                        }
                    }
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "ランダムヒント {len} 文字... => `{hint}` ",
                                len = num,
                                hint = hit_str.join(""),
                            ),
                        )
                        .expect("fail to post");
                },
            }
        } else {
            match parser::hint(&mut args) {
                Err(err_msg) => {
                    msg.channel_id
                        .say(&ctx, format!("{}", err_msg))
                        .expect("fail to post");
                },
                Ok(_) => {
                    msg.channel_id
                        .say(&ctx, "問題が出てないですよ？")
                        .expect("fail to post");
                },
            }
        }
    }
    Ok(())
}

#[command]
pub fn help(ctx: &mut Context, msg: &Message) -> CommandResult {
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
