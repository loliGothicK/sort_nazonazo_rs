use serenity::{model::channel::Message, prelude::*};

use super::super::bot;
use super::super::bot::ContestData;
use super::super::dictionary;
use super::super::sort::Sorted;
use indexmap::IndexMap;

use crate::try_say;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::str::from_utf8;

pub(crate) fn prob(ctx: &mut Context, msg: &Message, lang: bot::Lang) -> String {
    let dic = match lang {
        bot::Lang::En => &*dictionary::ENGLISH,
        bot::Lang::Ja => &*dictionary::JAPANESE,
        bot::Lang::Fr => &*dictionary::FRENCH,
        bot::Lang::De => &*dictionary::GERMAN,
        bot::Lang::It => &*dictionary::ITALIAN,
        bot::Lang::Ru => &*dictionary::RUSSIAN,
        bot::Lang::Eo => &*dictionary::ESPERANTO,
    };
    let ans = dic.get(&mut rand::thread_rng());
    let sorted = ans.sorted();
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
    println!("called prob: [{}, {}]", ans, sorted);
    ans.clone()
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
                try_say!(ctx, msg, "ヒィンｗ");
            } else {
                try_say!(ctx, msg, from_utf8(output.stderr.as_slice()).unwrap());
            }
        }
        Err(e) => {
            try_say!(ctx, msg, format!("{:?}", e));
        }
    }
    Ok(())
}

pub(crate) fn answer_check(ctx: &mut Context, msg: &Message) {
    if let Ok(mut quiz_guard) = bot::QUIZ.lock() {
        match quiz_guard.answer_check(&msg.content) {
            bot::CheckResult::WA => {
                // includes the case that bot is standing by.
                return;
            }
            bot::CheckResult::Assumed(_ans) => {
                if quiz_guard.is_holding() {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、正解です！\n正解は\"{}\"でした！ [{:.3} sec]",
                                &msg.author.name,
                                quiz_guard.ans().unwrap(),
                                quiz_guard.elapsed().unwrap(),
                            ),
                        )
                        .expect("fail to post");
                    *quiz_guard = bot::Status::StandingBy;
                    return;
                } else if quiz_guard.is_contesting() {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!(
                                "{} さん、正解です！\n正解は\"{}\"でした！ [{:.3} sec]",
                                &msg.author.name,
                                quiz_guard.ans().unwrap(),
                                quiz_guard.elapsed().unwrap(),
                            ),
                        )
                        .expect("fail to post");
                    let contest_result = &mut *bot::CONTEST_REUSLT.lock().unwrap();

                    *contest_result
                        .entry(msg.author.name.clone())
                        .or_insert(ContestData::default()) += (1, quiz_guard.elapsed().unwrap());

                    let (_, num) = quiz_guard.get_contest_num().unwrap();

                    if quiz_guard.is_contest_end() {
                        msg.channel_id
                            .say(
                                &ctx,
                                format!(
                                    "{num}問連続のコンテストが終了しました。\n{result}",
                                    num = num,
                                    result = bot::aggregates(contest_result)
                                ),
                            )
                            .expect("fail to post");
                        *contest_result = IndexMap::new();
                        *quiz_guard = bot::Status::StandingBy;
                    } else {
                        quiz_guard.contest_continue(ctx, msg);
                    }
                }
            }
            bot::CheckResult::Anagram(ans) => {
                *bot::CONTEST_REUSLT
                    .lock()
                    .unwrap()
                    .entry(msg.author.name.clone())
                    .or_insert(ContestData::default()) += (1, quiz_guard.elapsed().unwrap());
                msg.channel_id
                    .say(
                        &ctx,
                        format!(
                            "{} さん、{} は非想定解ですが正解です！",
                            &msg.author.name,
                            ans.to_lowercase()
                        ),
                    )
                    .expect("fail to post");
            }
            bot::CheckResult::Full(ans) => {
                *bot::CONTEST_REUSLT
                    .lock()
                    .unwrap()
                    .entry(msg.author.name.clone())
                    .or_insert(ContestData::default()) += (1, quiz_guard.elapsed().unwrap());
                msg.channel_id
                    .say(
                        &ctx,
                        format!(
                            "{} さん、{} は出題辞書にない非想定解ですが正解です！",
                            &msg.author.name,
                            ans.to_lowercase()
                        ),
                    )
                    .expect("fail to post");
            }
        }
    }
}
