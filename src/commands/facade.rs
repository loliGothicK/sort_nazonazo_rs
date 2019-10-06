use regex::Regex;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::channel::Message,
    prelude::*,
};
use std::time::Instant;

use super::super::bot;
use super::super::error::BotError;
use super::super::settings;
use super::super::sort::Sorted;
use super::{executors, parser};
use quick_error::ResultExt;
use std::env;
use std::fs::File;
use std::io::Write;
use unicode_segmentation::UnicodeSegmentation;

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
                &vec!["^(contest|", stringify!($command), $("|", stringify!($commands),)* ")$"].join("")
            ).unwrap();
        }
    };
}

quiz_commands!(en, ja, fr, de, it, ru);
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

group!({
    name: "settings",
    options: {},
    commands: [enable, disable],
});

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[command]
pub fn en(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~en' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let ans = executors::prob(ctx, &msg, bot::Lang::En);
            *guard = bot::Status::Holding(ans.clone(), bot::Lang::En, Instant::now());
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
            let ans = executors::prob(ctx, &msg, bot::Lang::Ja);
            *guard = bot::Status::Holding(ans.clone(), bot::Lang::Ja, Instant::now());
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
            let ans = executors::prob(ctx, &msg, bot::Lang::Fr);
            *guard = bot::Status::Holding(ans.clone(), bot::Lang::Fr, Instant::now());
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
            let ans = executors::prob(ctx, &msg, bot::Lang::De);
            *guard = bot::Status::Holding(ans.clone(), bot::Lang::De, Instant::now());
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
            let ans = executors::prob(ctx, &msg, bot::Lang::It);
            *guard = bot::Status::Holding(ans.clone(), bot::Lang::It, Instant::now());
        }
    }
    Ok(())
}
#[command]
pub fn ru(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~ru' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut guard) = bot::QUIZ.lock();
        then {
            let ans = executors::prob(ctx, &msg, bot::Lang::Ru);
            *guard = bot::Status::Holding(ans.clone(), bot::Lang::Ru, Instant::now());
        }
    }
    Ok(())
}

fn giveup_impl(ctx: &mut Context, msg: &Message, quiz_stat: &mut bot::Status) -> CommandResult {
    if !msg.author.bot {
        if quiz_stat.is_standing_by() {
            msg.channel_id
                .say(&ctx, "現在問題は出ていません。")
                .expect("fail to post");
        } else if quiz_stat.is_holding() {
            msg.channel_id
                .say(
                    &ctx,
                    format!("正解は \"{}\" でした...", quiz_stat.ans().unwrap()),
                )
                .expect("fail to post");
            *quiz_stat = bot::Status::StandingBy;
        } else {
            let contest_result = &mut *bot::CONTEST_REUSLT.lock().unwrap();
            *contest_result.entry("~giveup".to_string()).or_insert(0) += 1;
            if !quiz_stat.is_contest_end() {
                msg.channel_id
                    .say(
                        &ctx,
                        format!("正解は \"{}\" でした...", quiz_stat.ans().unwrap()),
                    )
                    .expect("fail to post");
                quiz_stat.contest_continue(ctx, &msg);
            } else {
                let (_, num) = quiz_stat.get_contest_num().unwrap();
                msg.channel_id
                    .say(
                        &ctx,
                        format!(
                            "正解は \"{ans}\" でした...\n{num}問連続のコンテストが終了しました。\n{result}",
                            ans = quiz_stat.ans().unwrap(),
                            num = num,
                            result = contest_result
                                .into_iter()
                                .map(|tuple| format!("{} AC: {}\n", tuple.1, tuple.0))
                                .collect::<String>()
                        ),
                    )
                    .expect("fail to post");
                *contest_result = std::collections::BTreeMap::new();
                *quiz_stat = bot::Status::StandingBy;
            }
        }
    }
    Ok(())
}

#[command]
pub fn giveup(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~giveup' by user '{}'", msg.author.name);
    if let Ok(mut guard) = bot::QUIZ.lock() {
        println!("giveup is accepted");
        giveup_impl(ctx, msg, &mut *guard)?;
    }
    Ok(())
}

#[command]
pub fn contest(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    use crate::bot::CONTEST_LIBRARY;
    println!("Got command '~contest' by user '{}'", msg.author.name);
    if_chain! {
        if !msg.author.bot;
        if let Ok(mut quiz_guard) = bot::QUIZ.lock();
        if quiz_guard.is_standing_by();
        then {
            match parser::contest(&mut args) {
                Err(err_msg) => {
                    msg.channel_id.say(&ctx, err_msg).expect("fail to post");
                    return Ok(());
                }
                Ok((num, mut languages)) => {
                    languages.sort();
                    languages.dedup();
                    CONTEST_LIBRARY.lock().unwrap().set(languages);
                    let (dic, lang) = CONTEST_LIBRARY
                        .lock()
                        .unwrap()
                        .select(&mut rand::thread_rng());
                    let ans = dic.get(&mut rand::thread_rng());
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{number}問のコンテストを始めます。\n問 1 (1/{number})\nソートなぞなぞ ソート前の {symbol} な〜んだ？\n{prob}",
                                number = num,
                                prob = ans.sorted(),
                                symbol = lang.as_symbol(),
                            ),
                        )
                        .expect("fail to post");
                    *quiz_guard = bot::Status::Contesting(ans.to_string(), lang, (1, num));
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
        if let (Ok(mut quiz), Ok(mut result)) = (bot::QUIZ.lock(), bot::CONTEST_REUSLT.lock()) {
            if quiz.is_contesting() {
                msg.channel_id
                    .say(&ctx, "コンテストを中止します。")
                    .expect("fail to post");
                *quiz = bot::Status::StandingBy;
                *result = std::collections::BTreeMap::new();
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

    ~hint {{NUM>=answer length}} [Optioin: --random]:
    => 答えの最初のNUM文字をヒントとして見ることができます。

USAGE [CONTEST]:
    ~contest {{NUM<=100}} {{LANGUAGES}}...
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

fn sync_setting() -> Result<(), BotError> {
    use quick_error::ResultExt;
    let path = std::path::Path::new("/tmp/settings/settings.toml");
    let mut conf = File::create(&path).context(path)?;
    conf.write_all(
        toml::to_string(&*settings::SETTINGS.lock().unwrap())
            .context("/tmp/settings/settings.toml")?
            .as_bytes(),
    )
    .context(path)?;
    conf.sync_all().context(path)?;
    Ok(())
}

#[command]
pub fn enable(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~enable' by user '{}'", msg.author.name);
    settings::SETTINGS
        .lock()
        .unwrap()
        .channel
        .enabled
        .push(*msg.channel_id.as_u64());
    msg.channel_id
        .say(&ctx, "このチャンネルでソートなぞなぞが有効になりました。")
        .expect("fail to post");
    Ok(sync_setting()?)
}

#[command]
pub fn disable(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~disable' by user '{}'", msg.author.name);
    settings::SETTINGS
        .lock()
        .unwrap()
        .channel
        .enabled
        .retain(|id| *id != *msg.channel_id.as_u64());
    msg.channel_id
        .say(&ctx, "このチャンネルでソートなぞなぞが無効になりました。")
        .expect("fail to post");
    Ok(sync_setting()?)
}
