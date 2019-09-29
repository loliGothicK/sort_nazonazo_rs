
use serenity::{
    client::Client,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult, StandardFramework,
    },
    model::{channel::Message, gateway::Ready},
    prelude::*,
};



use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::iter::FromIterator;
use std::str::from_utf8;

use super::super::bot;
use super::super::dictionary;

pub(crate) fn prob(
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

pub(crate) fn contest_continue(
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

pub(crate) fn kick(ctx: &mut Context, msg: &Message) -> std::io::Result<()> {
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

pub(crate) fn answer_check(ctx: &mut Context, msg: &Message) {
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
