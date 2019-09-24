//#![feature(async_await)]
use rand::distributions::{Distribution, Uniform};
use std::env;
#[macro_use]
extern crate lazy_static;
extern crate serde_derive;
extern crate toml;
use itertools::sorted;
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
    commands: [contest],
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
        Contesting(String, String, Lang, u32),
    }

    lazy_static! {
        pub static ref QUIZ: RwLock<Status> = RwLock::new(Status::StandingBy);
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
                {
                    match &*bot::QUIZ.read().unwrap() {
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
                let flag = {
                    match &*bot::QUIZ.read().unwrap() {
                        bot::Status::Holding(ref ans, _, _)
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
                            Err(true)
                        }
                        bot::Status::Contesting(ref ans, _, lang, count)
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
                            Ok((*lang, count - 1))
                        }
                        _ => Err(false),
                    }
                };

                if let Ok((_, count)) = flag {
                    if count == 0 {
                        if let Ok(guard) = bot::CONTEST_REUSLT.read() {
                            let mut v = Vec::from_iter(guard.iter());
                            v.sort_by(|&(_, a), &(_, b)| b.cmp(&a));
                            msg.channel_id
                                .say(&ctx, "コンテストが終了しました。")
                                .expect("fail to post");
                            for (name, ac) in &v {
                                msg.channel_id
                                    .say(&ctx, format!("{} AC: {}", ac, name))
                                    .expect("fail to post");
                            }
                            drop(v);
                        }
                        *bot::QUIZ.write().unwrap() = bot::Status::StandingBy;
                        *bot::CONTEST_REUSLT.write().unwrap() = std::collections::BTreeMap::new();
                        return;
                    }
                }
                match flag {
                    Ok((lang, count)) => {
                        let (ans, sorted) = prob(ctx, &msg, lang);
                        *bot::QUIZ.write().unwrap() =
                            bot::Status::Contesting(ans.clone(), sorted.clone(), lang, count);
                    }
                    Err(true) => *bot::QUIZ.write().unwrap() = bot::Status::StandingBy,
                    _ => {}
                };
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
        *bot::QUIZ.write().unwrap() =
            bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::En);
    };
    Ok(())
}

#[command]
fn ja(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let (ans, sorted) = prob(ctx, &msg, bot::Lang::Ja);
        *bot::QUIZ.write().unwrap() =
            bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::Ja);
    };
    Ok(())
}
#[command]
fn fr(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let (ans, sorted) = prob(ctx, &msg, bot::Lang::Fr);
        *bot::QUIZ.write().unwrap() =
            bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::Fr);
    };
    Ok(())
}
#[command]
fn de(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let (ans, sorted) = prob(ctx, &msg, bot::Lang::De);
        *bot::QUIZ.write().unwrap() =
            bot::Status::Holding(ans.clone(), sorted.clone(), bot::Lang::De);
    };
    Ok(())
}

#[command]
fn giveup(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let flag: Option<(bot::Lang, u32)> = match &*bot::QUIZ.write().unwrap() {
            bot::Status::Holding(ans, _, _) => {
                msg.channel_id
                    .say(&ctx, format!("正解は \"{}\" でした...", ans))
                    .expect("fail to post");
                None
            }
            bot::Status::Contesting(ans, _, lang, count) => {
                msg.channel_id
                    .say(&ctx, format!("正解は \"{}\" でした...", ans))
                    .expect("fail to post");
                Some((*lang, count - 1))
            }
            bot::Status::StandingBy => {
                msg.channel_id
                    .say(&ctx, "現在問題は出ていません。")
                    .expect("fail to post");
                None
            }
        };

        if let Some((_, count)) = flag {
            if count == 0 {
                msg.channel_id
                    .say(&ctx, "コンテストが終了しました。")
                    .expect("fail to post");
                *bot::QUIZ.write().unwrap() = bot::Status::StandingBy;
                return Ok(());
            }
        }

        match flag {
            Some((lang, count)) => {
                let (ans, sorted) = prob(ctx, &msg, lang);
                *bot::QUIZ.write().unwrap() =
                    bot::Status::Contesting(ans.clone(), sorted.clone(), lang, count);
            }
            None => *bot::QUIZ.write().unwrap() = bot::Status::StandingBy,
        }
    }
    Ok(())
}

#[command]
fn contest(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    if !msg.author.bot {
        match (args.single::<String>(), args.single::<u32>()) {
            (Ok(lang), Ok(num)) => {
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
                            "コンテストを始めます。\nソートなぞなぞ ソート前の文字列な〜んだ？\n{}",
                            sorted
                        ),
                    )
                    .expect("fail to post");
                *bot::QUIZ.write().unwrap() =
                    bot::Status::Contesting(ans.clone(), sorted.clone(), lang, num);
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
