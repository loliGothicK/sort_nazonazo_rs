//#![feature(async_await)]
use rand::distributions::{Distribution, Uniform};
use std::{env};
#[macro_use]
extern crate lazy_static;
extern crate serde_derive;
extern crate toml;
use std::iter::FromIterator;
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



group!({
    name: "quiz",
    options: {},
    commands: [en, ja, fr, de, giveup],
});

group!({
    name: "contest",
    options: {
        prefixes: ["contest"],
    },
    commands: [eng/*, jpn, fre, ger, ita*/],
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
    use std::{
        env,
        path::Path,
    };

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

pub mod bot_activity {

    
    use std::sync::RwLock;
    use std::collections::BTreeMap;

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

fn main() {
    // Login with a bot token from the environment
    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("token"), Handler)
        .expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
            .before(|ctx, msg, command_name| {
                println!("Got command '{}' by user '{}'",
                         command_name,
                         msg.author.name);
                if command_name == "en" || command_name == "ja" || command_name == "fr" || command_name == "de" || command_name == "it" {
                    match &*bot_activity::QUIZ.read().unwrap() {
                        bot_activity::Status::Holding(_, ref sorted, _) => {
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!("前回の出題が解かれていません\n問題: {}", sorted),
                                )
                                .expect("fail to post");
                            false
                        },
                        bot_activity::Status::Contesting(_, ref sorted, _, _) => {
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!("現在コンテスト中です\n問題: {}", sorted),
                                )
                                .expect("fail to post");
                            false
                        },
                        bot_activity::Status::StandingBy => true,
                    }
                } else { true }
            })
            .normal_message(|ctx, msg| {
                println!("got message \"{}\"", &msg.content);
                let flag = {
                    match &*bot_activity::QUIZ.read().unwrap() {
                        bot_activity::Status::Holding(ref ans, _, _) if &ans.to_lowercase() == &msg.content.to_lowercase() => {
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
                        },
                        bot_activity::Status::Contesting(ref ans, _, lang, count) if &ans.to_lowercase() == &msg.content.to_lowercase() => {
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
                            *bot_activity::CONTEST_REUSLT.write().unwrap().entry(msg.author.name.clone()).or_insert(0) += 1;
                            Ok((*lang, count-1))
                        },
                        _ => Err(false),
                    }
                };

                if let Ok((_, count)) = flag {
                    if count == 0 {
                        if let Ok(guard) = bot_activity::CONTEST_REUSLT.read() {
                            let mut v = Vec::from_iter(guard.iter());
                            v.sort_by(|&(_, a), &(_, b)| b.cmp(&a));
                            msg.channel_id
                                .say(
                                    &ctx,
                                    "コンテストが終了しました。",
                                )
                                .expect("fail to post");
                            for (name, ac) in &v {
                                msg.channel_id
                                    .say(
                                        &ctx,
                                        format!("{} AC: {}", ac, name),
                                    )
                                    .expect("fail to post");
                            }
                            drop(v);
                        }
                        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::StandingBy;
                        *bot_activity::CONTEST_REUSLT.write().unwrap() = std::collections::BTreeMap::new();
                        return;
                    }
                }

                match flag {
                    Ok((bot_activity::Lang::En, count)) => {
                        let gen = Uniform::new_inclusive(0, dictionary::ENGLISH.len() - 1);
                        let (ans, sorted) = dictionary::ENGLISH
                            .get_index(gen.sample(&mut rand::thread_rng()))
                            .unwrap();
                        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::En, count);
                        msg.channel_id
                            .say(
                                &ctx,
                                format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                            .expect("fail to post");
                    },
                    Ok((bot_activity::Lang::Fr, count)) => {
                        let gen = Uniform::new_inclusive(0, dictionary::FRENCH.len() - 1);
                        let (ans, sorted) = dictionary::FRENCH
                            .get_index(gen.sample(&mut rand::thread_rng()))
                            .unwrap();
                        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::Fr, count);
                        msg.channel_id
                            .say(
                                &ctx,
                                format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                            .expect("fail to post");
                    },
                    Ok((bot_activity::Lang::Ja, count)) => {
                        let gen = Uniform::new_inclusive(0, dictionary::JAPANESE.len() - 1);
                        let (ans, sorted) = dictionary::JAPANESE
                            .get_index(gen.sample(&mut rand::thread_rng()))
                            .unwrap();
                        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::Ja, count);
                        msg.channel_id
                            .say(
                                &ctx,
                                format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                            .expect("fail to post");
                    },
                    Ok((bot_activity::Lang::De, count)) => {
                        let gen = Uniform::new_inclusive(0, dictionary::GERMAN.len() - 1);
                        let (ans, sorted) = dictionary::GERMAN
                            .get_index(gen.sample(&mut rand::thread_rng()))
                            .unwrap();
                        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::De, count);
                        msg.channel_id
                            .say(
                                &ctx,
                                format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                            .expect("fail to post");
                    },
                    Ok((bot_activity::Lang::It, count)) => {
                        let gen = Uniform::new_inclusive(0, dictionary::ITALIAN.len() - 1);
                        let (ans, sorted) = dictionary::ITALIAN
                            .get_index(gen.sample(&mut rand::thread_rng()))
                            .unwrap();
                        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::It, count);
                        msg.channel_id
                            .say(
                                &ctx,
                                format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                            .expect("fail to post");
                    },
                    Err(true) => *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::StandingBy,
                    _ => {},
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
        let gen = Uniform::new_inclusive(0, dictionary::ENGLISH.len() - 1);
        let (ans, sorted) = dictionary::ENGLISH
            .get_index(gen.sample(&mut rand::thread_rng()))
            .unwrap();
        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Holding(ans.clone(), sorted.clone(), bot_activity::Lang::En);
        msg.channel_id
            .say(
                &ctx,
                format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),
            )
            .expect("fail to post");
    };

    Ok(())
}

#[command]
fn ja(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let gen = Uniform::new_inclusive(0, dictionary::JAPANESE.len() - 1);
        let (ans, sorted) = dictionary::JAPANESE
            .get_index(gen.sample(&mut rand::thread_rng()))
            .unwrap();
        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Holding(ans.clone(), sorted.clone(), bot_activity::Lang::Ja);
        msg.channel_id
            .say(
                &ctx,
                format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),
            )
            .expect("fail to post");
    };

    Ok(())
}
#[command]
fn fr(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let gen = Uniform::new_inclusive(0, dictionary::FRENCH.len() - 1);
        let (ans, sorted) = dictionary::FRENCH
            .get_index(gen.sample(&mut rand::thread_rng()))
            .unwrap();
        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Holding(ans.clone(), sorted.clone(), bot_activity::Lang::Fr);
        msg.channel_id
            .say(
                &ctx,
                format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),
            )
            .expect("fail to post");
    };

    Ok(())
}
#[command]
fn de(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let gen = Uniform::new_inclusive(0, dictionary::GERMAN.len() - 1);
        let (ans, sorted) = dictionary::GERMAN
            .get_index(gen.sample(&mut rand::thread_rng()))
            .unwrap();
        *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Holding(ans.clone(), sorted.clone(), bot_activity::Lang::De);
        msg.channel_id
            .say(
                &ctx,
                format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),
            )
            .expect("fail to post");
    };

    Ok(())

}

#[command]
fn giveup(ctx: &mut Context, msg: &Message) -> CommandResult {
    if !msg.author.bot {
        let flag: Option<(bot_activity::Lang, u32)> = match &*bot_activity::QUIZ.write().unwrap() {
            bot_activity::Status::Holding(ans, _, _) => {
                msg.channel_id
                    .say(&ctx, format!("正解は \"{}\" でした...", ans))
                    .expect("fail to post");
                None
            },
            bot_activity::Status::Contesting(ans, _, lang, count) => {
                msg.channel_id
                    .say(&ctx, format!("正解は \"{}\" でした...", ans))
                    .expect("fail to post");
                Some((*lang, count-1))
            },
            bot_activity::Status::StandingBy => {
                msg.channel_id
                    .say(&ctx, "現在問題は出ていません。")
                    .expect("fail to post");
                None
            },
        };

        if let Some((_, count)) = flag {
            if count == 0 {
                msg.channel_id
                    .say(
                        &ctx,
                        "コンテストが終了しました。",
                    )
                    .expect("fail to post");
                *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::StandingBy;
                return Ok(());
            }
        }

        match flag {
            Some((bot_activity::Lang::En, count)) => {
                let gen = Uniform::new_inclusive(0, dictionary::ENGLISH.len() - 1);
                let (ans, sorted) = dictionary::ENGLISH
                    .get_index(gen.sample(&mut rand::thread_rng()))
                    .unwrap();
                *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::En, count);
                msg.channel_id
                    .say(
                        &ctx,
                        format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                    .expect("fail to post");
            },
            Some((bot_activity::Lang::Fr, count)) => {
                let gen = Uniform::new_inclusive(0, dictionary::FRENCH.len() - 1);
                let (ans, sorted) = dictionary::FRENCH
                    .get_index(gen.sample(&mut rand::thread_rng()))
                    .unwrap();
                *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::Fr, count);
                msg.channel_id
                    .say(
                        &ctx,
                        format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                    .expect("fail to post");
            },
            Some((bot_activity::Lang::Ja, count)) => {
                let gen = Uniform::new_inclusive(0, dictionary::JAPANESE.len() - 1);
                let (ans, sorted) = dictionary::JAPANESE
                    .get_index(gen.sample(&mut rand::thread_rng()))
                    .unwrap();
                *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::Ja, count);
                msg.channel_id
                    .say(
                        &ctx,
                        format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                    .expect("fail to post");
            },
            Some((bot_activity::Lang::De, count)) => {
                let gen = Uniform::new_inclusive(0, dictionary::GERMAN.len() - 1);
                let (ans, sorted) = dictionary::GERMAN
                    .get_index(gen.sample(&mut rand::thread_rng()))
                    .unwrap();
                *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::De, count);
                msg.channel_id
                    .say(
                        &ctx,
                        format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                    .expect("fail to post");
            },
            Some((bot_activity::Lang::It, count)) => {
                let gen = Uniform::new_inclusive(0, dictionary::ITALIAN.len() - 1);
                let (ans, sorted) = dictionary::ITALIAN
                    .get_index(gen.sample(&mut rand::thread_rng()))
                    .unwrap();
                *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::It, count);
                msg.channel_id
                    .say(
                        &ctx,
                        format!("ソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),                    )
                    .expect("fail to post");
            },
            None => *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::StandingBy,
        }
    }
    Ok(())
}

#[command]
#[bucket = "contest"]
fn eng(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    if !msg.author.bot {
        match args.single::<u32>() {
            Ok(num) => {
                let gen = Uniform::new_inclusive(0, dictionary::ENGLISH.len() - 1);
                let (ans, sorted) = dictionary::ENGLISH
                    .get_index(gen.sample(&mut rand::thread_rng()))
                    .unwrap();
                *bot_activity::QUIZ.write().unwrap() = bot_activity::Status::Contesting(ans.clone(), sorted.clone(), bot_activity::Lang::En, num);
                msg.channel_id
                    .say(
                        &ctx,
                        format!("コンテストを始めます。\nソートなぞなぞ ソート前の文字列な〜んだ？\n{}", sorted),
                    )
                    .expect("fail to post");
            },
            Err(e) => {
                msg.channel_id
                    .say(
                        &ctx,
                        format!("{:?}", e)
                    )
                    .expect("fail to post");
            }
        }

    };

    Ok(())
}
