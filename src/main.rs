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
#[macro_use]
extern crate quick_error;
extern crate ordinal;
//extern crate nazonazo_macros;

use regex::Regex;
use serenity::{
    client::Client, framework::standard::StandardFramework, model::gateway::Ready, prelude::*,
};
use std::env;

pub mod bot;
pub mod commands;
pub mod dictionary;
pub mod error;
pub mod settings;
pub mod sort;
use sort::Sorted;

use commands::{executors, facade};
use serenity::model::id::ChannelId;

#[macro_export]
macro_rules! try_say {
    ($ctx: expr, $msg: expr, $response: expr) => {
        if let Err(why) = ($msg).channel_id.say(&($ctx), $response) {
            println!("{}", why);
        }
    };
}

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, ready: Ready) {
        for id in &settings::SETTINGS.lock().unwrap().channel.enabled {
            ChannelId::from(*id)
                .say(&ctx, "おはようございます。 botの起動をおしらせします！")
                .expect("fail to send");
        }
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
            .bucket("basic", |b| b.delay(1).time_span(0).limit(1))
            .bucket("long", |b| b.delay(10).time_span(60).limit(1))
            .before(|ctx, msg, command_name| {
                let re = Regex::new(r"^enable$").unwrap();
                if re.is_match(command_name) {
                    return true;
                }
                if !settings::SETTINGS
                    .lock()
                    .unwrap()
                    .channel
                    .enabled
                    .contains(msg.channel_id.as_u64())
                {
                    return false;
                }
                if facade::QUIZ_COMMANDS_REGEX.is_match(&command_name.to_string()) {
                    match &*bot::QUIZ.lock().unwrap() {
                        bot::Status::Holding(ref ans, ..) => {
                            try_say!(
                                ctx,
                                msg,
                                format!("前回の出題が解かれていません\n問題: {}", ans.sorted())
                            );
                            false
                        }
                        bot::Status::Contesting(ref ans, ..) => {
                            try_say!(
                                ctx,
                                msg,
                                format!("現在コンテスト中です\n問題: {}", ans.sorted())
                            );
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
            .group(&commands::facade::SETTINGS_GROUP)
            .group(&commands::facade::EXTRA_GROUP)
            .help(&commands::facade::NAZONAZO_HELP),
    );

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}
