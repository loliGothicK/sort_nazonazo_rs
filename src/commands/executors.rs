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
    try_say!(
        ctx,
        msg,
        format!(
            "ソートなぞなぞ ソート前の {as_str} な〜んだ？\n`{prob}`",
            as_str = lang.as_symbol(),
            prob = sorted
        ));
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
        let elapsed = quiz_guard.elapsed();
        match quiz_guard.answer_check(&msg.content) {
            bot::CheckResult::WA => {
                // includes the case that bot is standing by.
                return;
            }
            bot::CheckResult::Assumed(_ans) => {
                if quiz_guard.is_holding() {
                    try_say!(
                        ctx,
                        msg,
                        format!(
                            "{} さん、正解です！\n正解は\"{}\"でした！ [{:.3} sec]",
                            &msg.author.name,
                            quiz_guard.ans().unwrap(),
                            elapsed.unwrap(),
                        )
                    );
                    *quiz_guard = bot::Status::StandingBy;
                    return;
                } else if quiz_guard.is_contesting() {
                    try_say!(
                        ctx,
                        msg,
                        format!(
                            "{} さん、正解です！\n正解は\"{}\"でした！ [{:.3} sec]",
                            &msg.author.name,
                            quiz_guard.ans().unwrap(),
                            elapsed.unwrap(),
                        )
                    );
                    let contest_result = &mut *bot::CONTEST_RESULT.lock().unwrap();

                    *contest_result
                        .entry(msg.author.name.clone())
                        .or_insert(ContestData::default()) += elapsed.unwrap();

                    let (_, num) = quiz_guard.get_contest_num().unwrap();

                    if quiz_guard.is_contest_end() {
                        try_say!(
                            ctx,
                            msg,
                            format!(
                                "{num}問連続のコンテストが終了しました。\n{result}",
                                num = num,
                                result = bot::aggregates(dbg!(&*contest_result))
                            )
                        );
                        *contest_result = IndexMap::new();
                        *quiz_guard = bot::Status::StandingBy;
                    } else {
                        quiz_guard.contest_continue(ctx, msg);
                    }
                }
            }
            bot::CheckResult::Anagram(ans) => {
                *bot::CONTEST_RESULT
                    .lock()
                    .unwrap()
                    .entry(msg.author.name.clone())
                    .or_insert(ContestData::default()) += elapsed.unwrap();
                try_say!(
                    ctx,
                    msg,
                    format!(
                        "{} さん、{} は非想定解ですが正解です！",
                        &msg.author.name,
                        ans.to_lowercase()
                    )
                );
            }
            bot::CheckResult::Full(ans) => {
                *bot::CONTEST_RESULT
                    .lock()
                    .unwrap()
                    .entry(msg.author.name.clone())
                    .or_insert(ContestData::default()) += elapsed.unwrap();
                try_say!(
                    ctx,
                    msg,
                    format!(
                        "{} さん、{} は出題辞書にない非想定解ですが正解です！",
                        &msg.author.name,
                        ans.to_lowercase()
                    )
                );
            }
        }
    }
}
