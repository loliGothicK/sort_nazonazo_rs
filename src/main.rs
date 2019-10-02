//#![feature(async_await)]
#![feature(result_map_or_else)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate if_chain;
#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;
extern crate clap;
extern crate regex;
extern crate toml;
extern crate unicode_segmentation;

//extern crate nazonazo_macros;

use regex::Regex;
use serenity::{
    client::Client, framework::standard::StandardFramework, model::gateway::Ready, prelude::*,
};
use std::env;

pub mod bot;
pub mod commands;
pub mod dictionary;
pub mod settings;
pub mod sort;
use sort::Sorted;

use commands::{executors, facade};

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
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
                if facade::QUIZ_COMMANDS_REGEX.is_match(&command_name.to_string()) {
                    match &*bot::QUIZ.lock().unwrap() {
                        bot::Status::Holding(ref ans, ..) => {
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!("前回の出題が解かれていません\n問題: {}", ans.sorted()),
                                )
                                .expect("fail to post");
                            false
                        }
                        bot::Status::Contesting(ref ans, ..) => {
                            msg.channel_id
                                .say(
                                    &ctx,
                                    format!("現在コンテスト中です\n問題: {}", ans.sorted()),
                                )
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
                if !msg.author.bot {
                    let re = Regex::new(r"^kick\(.*\);$").unwrap();
                    if re.is_match(&msg.content) {
                        println!("{:?}", executors::kick(ctx, msg));
                        return;
                    }
                    executors::answer_check(ctx, msg);
                }
            })
            .group(&commands::facade::QUIZ_GROUP)
            .group(&commands::facade::CONTEST_GROUP)
            .group(&commands::facade::HELP_GROUP)
            .group(&commands::facade::EXTRA_GROUP),
    );

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}
