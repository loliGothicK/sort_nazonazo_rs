//#![feature(async_await)]
use rand::distributions::{Distribution, Uniform};
use std::env;
#[macro_use]
extern crate lazy_static;
extern crate serde_derive;
extern crate toml;
extern crate unicode_segmentation;

use unicode_segmentation::UnicodeSegmentation;
//extern crate nazonazo_macros;

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

use std::iter::FromIterator;

pub mod dictionary;

macro_rules! count {
    ( $x:ident ) => (1usize);
    ( $x:ident, $($xs:tt)* ) => (1usize + count!($($xs)*));
}

macro_rules! quiz_commands {
    () => {};
    ( $( $commands:ident ),+ ) => {
        group!({
            name: "quiz",
            options: {},
            commands: [$($commands),+],
        });
        const COMMAND_NUM: usize = count!($($commands),+);
        lazy_static! {
            pub static ref QUIZ_COMMANDS: [String; COMMAND_NUM] = [$(stringify!($commands).to_string(),)+];
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

pub mod bot {

    use std::collections::BTreeMap;
    use std::sync::RwLock;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Copy)]
    pub enum Lang {
        En,
        Ja,
        Fr,
        De,
        It,
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

    lazy_static! {
        pub static ref QUIZ: Arc<Mutex<Status>> = Arc::new(Mutex::new(Status::StandingBy));
        pub static ref CONTEST_REUSLT: RwLock<BTreeMap<String, u32>> = RwLock::new(BTreeMap::new());
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
    let gen = Uniform::new_inclusive(0, dic.len() - 1);
    let (ans, sorted) = dic.get_index(gen.sample(&mut rand::thread_rng())).unwrap();
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
    let gen = Uniform::new_inclusive(0, dic.len() - 1);
    let (ans, sorted) = dic.get_index(gen.sample(&mut rand::thread_rng())).unwrap();
    msg.channel_id
        .say(
            &ctx,
            format!(
                "問 {current} ({current}/{number})\nソートなぞなぞ ソート前の文字列な〜んだ？\n{prob}",
                number = num,
                current = count,
                prob = sorted
            ),
        )
        .expect("fail to post");
    (ans.clone(), sorted.clone())
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
                        bot::Status::Holding(_, ref sorted, _) => {
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!("前回の出題が解かれていません\n問題: {}", sorted),
                                )
                                .expect("fail to post");
                            false
                        }
                        bot::Status::Contesting(_, ref sorted, _, _) => {
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
                                            format!("{}問連続のコンテストが終了しました。", num),
                                        )
                                        .expect("fail to post");
                                    for (name, ac) in &v {
                                        msg.channel_id
                                            .say(&ctx, format!("{} AC: {}", ac, name))
                                            .expect("fail to post");
                                    }
                                    drop(v);
                                }
                                *bot::CONTEST_REUSLT.write().unwrap() =
                                    std::collections::BTreeMap::new();
                                *guard = bot::Status::StandingBy;
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

#[command]
fn giveup(ctx: &mut Context, msg: &Message) -> CommandResult {
    println!("Got command '~giveup' by user '{}'", msg.author.name);
    if !msg.author.bot {
        let flag: Option<(bot::Lang, (u32, u32))> = match &*bot::QUIZ.lock().unwrap() {
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
        };

        if let Some((_, (count, num))) = flag {
            if count > num {
                msg.channel_id
                    .say(&ctx, format!("{}問連続のコンテストが終了しました。", num))
                    .expect("fail to post");
                loop {
                    if let Ok(mut guard) = bot::QUIZ.lock() {
                        *guard = bot::Status::StandingBy;
                        break;
                    }
                }
                return Ok(());
            }
        }

        match flag {
            Some((lang, (count, num))) => {
                let (ans, sorted) = contest_continue(ctx, &msg, lang, count, num);
                loop {
                    if let Ok(mut guard) = bot::QUIZ.lock() {
                        *guard = bot::Status::Contesting(
                            ans.clone(),
                            sorted.clone(),
                            lang,
                            (count, num),
                        );
                        break;
                    }
                }
            }
            None => loop {
                if let Ok(mut guard) = bot::QUIZ.lock() {
                    *guard = bot::Status::StandingBy;
                    break;
                }
            },
        }
    }
    Ok(())
}

#[command]
fn contest(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    println!("Got command '~contest' by user '{}'", msg.author.name);
    let first = args.single::<String>();
    let second = args.single::<u32>();
    if !msg.author.bot {
        match (first, second) {
            (Ok(lang), Ok(mut num)) => {
                if num == 0 {
                    msg.channel_id
                        .say(&ctx, "0問のコンテストは開催できません！")
                        .expect("fail to post");
                    return Ok(());
                }
                if num > 100 {
                    msg.channel_id
                        .say(&ctx, format!("{}問は多すぎるので100問にしますね！", num))
                        .expect("fail to post");
                    num = 100;
                }
                let (dic, lang) = match lang {
                    en if en == "en" => (&*dictionary::ENGLISH, bot::Lang::En),
                    ja if ja == "ja" => (&*dictionary::JAPANESE, bot::Lang::Ja),
                    fr if fr == "fr" => (&*dictionary::FRENCH, bot::Lang::Fr),
                    de if de == "de" => (&*dictionary::GERMAN, bot::Lang::De),
                    it if it == "it" => (&*dictionary::ITALIAN, bot::Lang::It),
                    _ => {
                        return Ok(());
                    }
                };

                let gen = Uniform::new_inclusive(0, dic.len() - 1);
                let (ans, sorted) = dic.get_index(gen.sample(&mut rand::thread_rng())).unwrap();
                msg.channel_id
                    .say(
                        &ctx,
                        format!(
                            "{number}問のコンテストを始めます。\n問 1 (1/{number})\nソートなぞなぞ ソート前の文字列な〜んだ？\n{prob}",
                            number = num,
                            prob = sorted
                        ),
                    )
                    .expect("fail to post");
                loop {
                    if let Ok(mut guard) = bot::QUIZ.lock() {
                        *guard =
                            bot::Status::Contesting(ans.clone(), sorted.clone(), lang, (1, num));
                        break;
                    }
                }
            }
            (Err(e), Err(f)) => {
                msg.channel_id
                    .say(&ctx, format!("{:?}", e))
                    .expect("fail to post");
                msg.channel_id
                    .say(&ctx, format!("{:?}", f))
                    .expect("fail to post");
            }
            (_, Err(f)) => {
                msg.channel_id
                    .say(&ctx, format!("{:?}", f))
                    .expect("fail to post");
            }
            (Err(e), _) => {
                msg.channel_id
                    .say(&ctx, format!("{:?}", e))
                    .expect("fail to post");
            }
        }
    };

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
        if let Ok(len) = args.single::<usize>() {
            loop {
                if let Ok(guard) = bot::QUIZ.lock() {
                    if guard.is_holding() || guard.is_contesting() {
                        let mut g =
                            UnicodeSegmentation::graphemes(guard.ans().unwrap().as_str(), true)
                                .collect::<Vec<&str>>();
                        if len < g.len() {
                            g.truncate(len);
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!(
                                        "{len}文字のヒント、答えの先頭 {len} 文字は...\n\"{hint}\"\nです！",
                                        len = len,
                                        hint = g.into_iter().collect::<String>(),
                                    ),
                                )
                                .expect("fail to post");
                        } else {
                            msg.channel_id
                                .say(&ctx, "ヒントが単語より長過いわ、ボケ")
                                .expect("fail to post");
                        }
                    } else {
                        msg.channel_id
                            .say(&ctx, "現在問題は出ていません。")
                            .expect("fail to post");
                    }
                    break;
                }
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
