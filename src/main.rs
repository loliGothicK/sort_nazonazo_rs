use std::{
    env,
    collections::{HashMap, BTreeMap},
    cell::RefCell,
    fmt::Write,
    sync::Arc,
};

use itertools::Itertools;
use indexmap::IndexMap;
use rand::distributions::{Uniform, Distribution};
use rand::prelude::*;

#[macro_use]
extern crate lazy_static;

use serenity::{
    client::{Client, bridge::gateway::{ShardId, ShardManager}},
    prelude::*,
    framework::standard::{
        Args,
        CheckResult,
        CommandGroup, CommandOptions, CommandResult, DispatchError, help_commands, HelpOptions, macros::{check, command, group, help},
        StandardFramework,
    },
    model::{
        channel::{Channel, Message},
        gateway::Ready,
        id::UserId,
    },
    utils::{content_safe, ContentSafeOptions, MessageBuilder},
};
use std::borrow::BorrowMut;

group!({
    name: "quiz",
    options: {},
    commands: [en],
});

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

pub mod status {
    use indexmap::IndexMap;
    use std::{fs, io::{BufRead, BufReader}};
    use itertools::Itertools;
    lazy_static! {
        pub static ref DICTIONARY: IndexMap<String, String> = {
            let mut dictionary = IndexMap::new();
            for line in BufReader::new(
                fs::File::open("/home/mitama/github/mitama-test-bot/dics/TOEIC.dic").unwrap(),
            )
            .lines()
            {
                if let Ok(word) = line {
                    dictionary.insert(
                        word.clone(),
                        word.clone()
                            .chars()
                            .into_iter()
                            .sorted()
                            .collect::<String>(),
                    );
                }
            }
            dictionary
        };
    }
}

pub mod bot_activity {

    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::result::Result;
    use std::sync::RwLock;

    lazy_static! {
        pub static ref QUIZ: RwLock<Result<(String, String), ()>> = {
            let mut e = Err(());
            RwLock::new(e)
        };
    }
}

fn main() {
    // Login with a bot token from the environment
    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("token"), Handler)
        .expect("Error creating client");
    client.with_framework(StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .normal_message(|ctx, msg| {
            println!("got message \"{}\"", &msg.content);
            let flag = {
                match &*bot_activity::QUIZ.read().unwrap() {
                    Ok((ref ans, _)) if ans == &msg.content => {
                        println!("pass Ok arm: ans = {}.", ans);
                        msg.channel_id.say(
                            ctx,
                            format!(
                                "{} さん、正解です！\n正解は\"{}\"でした！", &msg.author.name, &ans
                            ),
                        ).expect("fail to post");
                        true
                    },
                    _ => false
                }
            };

            if flag {
                *bot_activity::QUIZ.write().unwrap() = Err(());
            };
        })
        .group(&QUIZ_GROUP));

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
fn en(ctx: &mut Context, msg: &Message) -> CommandResult {

    if !msg.author.bot {
        let flag = {
            if let Ok((_, sorted)) = &*bot_activity::QUIZ.read().unwrap() {
                msg.channel_id.say(
                    &ctx,
                    format!(
                        "前回の出題が解かれていません\n問題: {}", sorted
                    ),
                ).expect("fail to post");
                false
            } else { true }
        };

        if flag {
            let gen = Uniform::new_inclusive(0, status::DICTIONARY.len());
            let mut rng = rand::thread_rng();
            let (ans, sorted) = status::DICTIONARY
                .get_index(gen.sample(&mut rand::thread_rng()))
                .unwrap();
            *bot_activity::QUIZ.write().unwrap() = Ok((ans.clone(), sorted.clone()));
            msg.channel_id.say(
                &ctx,
                format!(
                    "ソートなぞなぞ ソート前の文字列な〜んだ？\n {}", sorted
                ),
            ).expect("fail to post");
        }
    };

    Ok(())
}
