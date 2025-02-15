use std::env;
use std::env::VarError;

use commands::{
    goodbye::goodbye, guild::guild, help::help, ping::pong, starboard::starboard, tags::*,
    voting::*, welcome::welcome,
};

pub use database::Data;
use dotenvy::dotenv;
use poise::serenity_prelude::ActivityData;
use poise::serenity_prelude::Client;

use crate::event_handlers::event_handler;
use poise::serenity_prelude as serenity;
use poise::Prefix;
use serenity::{Color, GatewayIntents};

mod commands;
mod database;
mod event_handlers;

const EMBED_COLOR: Color = Color::from_rgb(255, 172, 51);

const INTENTS: GatewayIntents = GatewayIntents::non_privileged()
    .union(GatewayIntents::MESSAGE_CONTENT)
    .union(GatewayIntents::GUILD_MEMBERS);

type Context<'a> = poise::Context<'a, Data, Error>;
type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() {
    let commands = vec![
        register_commands(),
        help(),
        pong(),
        starboard(),
        guild(),
        welcome(),
        goodbye(),
        tag_edit(),
        tag_list(),
        tags(),
        burg_vote(),
        silly_check(),
        count_reactions()
    ];

    match dotenv() {
        Ok(_) => (),
        Err(err) if err.not_found() => {
            if !not_using_dotenv() {
                println!("You have not included a .env file! If this is intentional you can disable this warning with `DISABLE_NO_DOTENV_WARNING=1`")
            }
        }
        Err(err) => panic!("Dotenv error: {}", err),
    }

    tracing_subscriber::fmt::init();

    // If we used dotenv! you would have to recompile to update these
    let token =
        env::var("DISCORD_TOKEN").expect("No discord token found in environment variables!");
    let (primary_prefix, addition_prefixes) = parse_prefixes();

    let data = database::init_data().await;

    let db = data.db.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: primary_prefix,
                additional_prefixes: addition_prefixes,
                edit_tracker: Some(
                    poise::EditTracker::for_timespan(std::time::Duration::from_secs(120)).into(),
                ),
                ..Default::default()
            },
            commands,
            event_handler: |ctx, event, _framework, data| Box::pin(event_handler(ctx, event, data)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                Ok(data)
            })
        })
        .build();

    let mut client = Client::builder(token, INTENTS)
        .activity(ActivityData::watching("C code become rusty"))
        .framework(framework)
        .await
        .unwrap();

    // ctrl+c handler
    let shard_handler = client.shard_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Couldn't register a ctrl+c handler!");
        tracing::info!("Shutting down oxibot!");
        shard_handler.shutdown_all().await;
        db.close().await;
    });

    tracing::info!("Starting oxibot!");
    client.start().await.unwrap();
}

fn not_using_dotenv() -> bool {
    match env::var("DISABLE_NO_DOTENV_WARNING")
        .map(|x| x.to_ascii_lowercase())
        .as_deref()
    {
        Ok("1" | "true") => true,
        Ok("0" | "false") => false,
        Ok(_) => panic!("DISABLE_NO_DOTENV_WARNING environment variable is not a valid value"),
        Err(VarError::NotPresent) => false,
        Err(VarError::NotUnicode(err)) => panic!(
            "DISABLE_NO_DOTENV_WARNING environment variable is set to valid Unicode, found: {:?}",
            err
        ),
    }
}

fn parse_prefixes() -> (Option<String>, Vec<Prefix>) {
    let unparsed = match env::var("PREFIXES") {
        Ok(unparsed) => unparsed,
        Err(VarError::NotPresent) => return (None, Vec::new()),
        _ => panic!("Could not handle the environment variable for prefixes"),
    };

    let mut split = unparsed.split(' ');

    let first = split
        .next()
        .expect("Could not parse prefixes from environment variables")
        .to_string();

    // We need to leak these strings since `Prefix::Literal` only accepts `&'static str` for some reason
    let split = split
        .map(|slice| Box::leak(slice.into()))
        .map(|leaked| Prefix::Literal(leaked));

    (Some(first), split.collect())
}

#[poise::command(prefix_command, hide_in_help, owners_only)]
async fn register_commands(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
