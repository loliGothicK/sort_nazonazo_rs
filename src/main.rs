//#![feature(async_await)]
use rand::distributions::{Distribution, Uniform};
use std::env;
#[macro_use]
extern crate lazy_static;
extern crate serde_derive;
extern crate toml;

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

group!({
    name: "quiz",
    options: {},
    commands: [en, ja, fr, de, giveup],
});

group!({
    name: "contest",
    options: {},
    commands: [contest, unrated],
});
struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

pub mod dictionary {
    use indexmap::IndexMap;
    use itertools::Itertools;
    use serde_derive::{Deserialize, Serialize};
    use std::fs::File;
    use std::io::Read;
    use std::{env, path::Path};

    #[derive(Serialize, Deserialize, Debug)]
    struct Dictionaries {
        pub questions: Vec<String>,
        pub full: Option<Vec<String>>,
    }

    pub enum Dictionary {
        Japanese(IndexMap<String, String>),
        English(IndexMap<String, String>),
        French(IndexMap<String, String>),
        German(IndexMap<String, String>),
        Italian(IndexMap<String, String>),
    }

    impl Dictionary {
        pub fn get_index(&self, idx: usize) -> Option<(&String, &String)> {
            match &self {
                Dictionary::Japanese(dic) => dic.get_index(idx),
                Dictionary::English(dic) => dic.get_index(idx),
                Dictionary::French(dic) => dic.get_index(idx),
                Dictionary::German(dic) => dic.get_index(idx),
                Dictionary::Italian(dic) => dic.get_index(idx),
            }
        }

        pub fn len(&self) -> usize {
            match &self {
                Dictionary::Japanese(dic) => dic.len(),
                Dictionary::English(dic) => dic.len(),
                Dictionary::French(dic) => dic.len(),
                Dictionary::German(dic) => dic.len(),
                Dictionary::Italian(dic) => dic.len(),
            }
        }
    }

    lazy_static! {
        pub static ref ENGLISH: Dictionary = {
            let mut f = match env::var("DIC_DIR") {
                Ok(path) => File::open(Path::new(&path).join("english.toml")).expect("file not found"),
                Err(e) => panic!(e),
            };
            let mut conf = String::new();
            // config file open
            // read config.toml
            let _ = f.read_to_string(&mut conf).unwrap();
            // parse toml
            let config: Dictionaries = toml::from_slice(conf.as_bytes()).unwrap();

            let mut dictionary = IndexMap::new();
            for word in config.questions {
                dictionary.insert(
                    word.clone(),
                    word.clone()
                        .chars()
                        .into_iter()
                        .sorted()
                        .collect::<String>(),
                );
            }
            Dictionary::English(dictionary)
        };
        pub static ref JAPANESE: Dictionary = {
            let mut conf = String::new();
            // config file open
            let mut f = env::var("DIC_DIR").map(|path| File::open(Path::new(&path).join("japanese.toml")).expect("file not found")).unwrap();
            // read config.toml
            let _ = f.read_to_string(&mut conf).unwrap();
            // parse toml
            let config: Dictionaries = toml::from_slice(conf.as_bytes()).unwrap();

            let mut dictionary = IndexMap::new();
            for word in config.questions {
                dictionary.insert(
                    word.clone(),
                    word.clone()
                        .chars()
                        .into_iter()
                        .sorted()
                        .collect::<String>(),
                );
            }
            Dictionary::Japanese(dictionary)
        };
        pub static ref FRENCH: Dictionary = {
            let mut conf = String::new();
            // config file open
            let mut f = env::var("DIC_DIR").map(|path| File::open(Path::new(&path).join("french.toml")).expect("file not found")).unwrap();
            // read config.toml
            let _ = f.read_to_string(&mut conf).unwrap();
            // parse toml
            let config: Dictionaries = toml::from_slice(conf.as_bytes()).unwrap();

            let mut dictionary = IndexMap::new();
            for word in config.questions {
                dictionary.insert(
                    word.clone(),
                    word.clone()
                        .chars()
                        .into_iter()
                        .sorted()
                        .collect::<String>(),
                );
            }
            Dictionary::French(dictionary)
        };
        pub static ref GERMAN: Dictionary = {
            let mut conf = String::new();
            // config file open
            let mut f = env::var("DIC_DIR").map(|path| File::open(Path::new(&path).join("german.toml")).expect("file not found")).unwrap();
            // read config.toml
            let _ = f.read_to_string(&mut conf).unwrap();
            // parse toml
            let config: Dictionaries = toml::from_slice(conf.as_bytes()).unwrap();

            let mut dictionary = IndexMap::new();
            for word in config.questions {
                dictionary.insert(
                    word.clone(),
                    word.clone()
                        .to_lowercase()
                        .chars()
                        .into_iter()
                        .sorted()
                        .collect::<String>(),
                );
            }
            Dictionary::German(dictionary)
        };
        pub static ref ITALIAN: Dictionary = {
            let mut conf = String::new();
            // config file open
            let mut f = env::var("DIC_DIR").map(|path| File::open(Path::new(&path).join("italian.toml")).expect("file not found")).unwrap();
            // read config.toml
            let _ = f.read_to_string(&mut conf).unwrap();
            // parse toml
            let config: Dictionaries = toml::from_slice(conf.as_bytes()).unwrap();

            let mut dictionary = IndexMap::new();
            for word in config.questions {
                dictionary.insert(
                    word.clone(),
                    word.clone()
                        .chars()
                        .into_iter()
                        .sorted()
                        .collect::<String>(),
                );
            }
            Dictionary::Italian(dictionary)
        };
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
    }

    lazy_static! {
        pub static ref QUIZ: Arc<Mutex<Status>> = Arc::new(Mutex::new(Status::StandingBy));
        pub static ref CONTEST_REUSLT: RwLock<BTreeMap<String, u32>> = RwLock::new(BTreeMap::new());
    }
}

fn prob(ctx: &mut Context, msg: &Message, lang: bot::Lang) -> (String, String) {
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
            format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),
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
                if command_name == "en"
                    || command_name == "ja"
                    || command_name == "fr"
                    || command_name == "de"
                    || command_name == "it"
                    || command_name == "contest"
                {
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
            .group(&CONTEST_GROUP),
    );

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
fn en(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let (ans, sorted) = prob(ctx, &msg, bot::Lang::En);
        *bot::QUIZ.lock().unwrap() =
            bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::En);
    };
    Ok(())
}

#[command]
fn ja(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let (ans, sorted) = prob(ctx, &msg, bot::Lang::Ja);
        *bot::QUIZ.lock().unwrap() =
            bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::Ja);
    };
    Ok(())
}
#[command]
fn fr(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let (ans, sorted) = prob(ctx, &msg, bot::Lang::Fr);
        *bot::QUIZ.lock().unwrap() =
            bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::Fr);
    };
    Ok(())
}
#[command]
fn de(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let (ans, sorted) = prob(ctx, &msg, bot::Lang::De);
        *bot::QUIZ.lock().unwrap() =
            bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::De);
    };
    Ok(())
}

#[command]
fn giveup(ctx: &mut Context, msg: &Message) -> CommandResult {
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
                *bot::QUIZ.lock().unwrap() = bot::Status::StandingBy;
                return Ok(());
            }
        }

        match flag {
            Some((lang, (count, num))) => {
                let (ans, sorted) = contest_continue(ctx, &msg, lang, count, num);
                *bot::QUIZ.lock().unwrap() =
                    bot::Status::Contesting(ans.clone(), sorted.clone(), lang, (count, num));
            }
            None => *bot::QUIZ.lock().unwrap() = bot::Status::StandingBy,
        }
    }
    Ok(())
}

#[command]
fn contest(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let first = args.single::<String>();
    let mut second = args.single::<u32>();
    if !msg.author.bot {
        match (first, second) {
            (Ok(lang), Ok(mut num)) => {
                if num == 0 {
                    msg.channel_id
                        .say(
                            &ctx,
                            "0問のコンテストは開催できません！"
                        )
                        .expect("fail to post");
                    return Ok(());
                }
                if num > 100 {
                    msg.channel_id
                        .say(
                            &ctx,
                            format!("{}問は多すぎるので100問にしますね！", num)
                        )
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
                *bot::QUIZ.lock().unwrap() =
                    bot::Status::Contesting(ans.clone(), sorted.clone(), lang, (1, num));
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
    if let Ok(mut guard) = bot::QUIZ.lock() {
        if guard.is_contesting() {
            msg.channel_id
                .say(
                    &ctx,
                    "コンテストを中止します。",
                )
                .expect("fail to post");
            *guard = bot::Status::StandingBy;
        }
        else {
            msg.channel_id
                .say(
                    &ctx,
                    "現在コンテストは開催されていません。",
                )
                .expect("fail to post");
        }
    }
    Ok(())
}
