use crate::database::IntoDatabase;
use crate::serenity::Channel;
use crate::{Context, Error};
use std::fmt::Write;

#[poise::command(prefix_command, slash_command, subcommands("message", "channel"))]
pub async fn goodbye(_ctx: Context<'_>, _arg: String) -> Result<(), Error> {
    Ok(())
}

#[poise::command(prefix_command, slash_command, subcommands("add", "list"))]
pub async fn message(_ctx: Context<'_>, _arg: String) -> Result<(), Error> {
    Ok(())
}

///Use `{}` to indicate where the username should be placed, otherwise it is placed at the end
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MANAGE_CHANNELS"
)]
pub async fn add(ctx: Context<'_>, message: String) -> Result<(), Error> {
    // SAFETY: Since this command is guild_only this should NEVER fail
    let guild = ctx.guild_id().unwrap().into_db();
    //blah
    sqlx::query!("UPDATE guild SET goodbye_messages = array_append(goodbye_messages, $1) WHERE guild.discord_id = $2", message, guild)
        .execute(&ctx.data().db)
        .await?;

    ctx.say("Done!").await?;

    Ok(())
}

///Lists all current goodbye messages
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    // SAFETY: Since this command is guild_only this should NEVER fail
    let guild = ctx.guild_id().unwrap().into_db();

    let goodbye_messages = sqlx::query!(
        "SELECT goodbye_messages FROM guild WHERE guild.discord_id = $1",
        guild
    )
    .fetch_one(&ctx.data().db)
    .await?
    .goodbye_messages;

    let mut formated_messages: String =
        goodbye_messages
            .into_iter()
            .fold(String::new(), |item, mut message| {
                writeln!(&mut message, "```\n{item}```").unwrap();
                message
            });

    formated_messages.pop();

    ctx.defer_ephemeral().await?;
    ctx.say(formated_messages).await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, subcommands("change"))]
pub async fn channel(_ctx: Context<'_>, _arg: String) -> Result<(), Error> {
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    guild_only,
    required_permissions = "MANAGE_CHANNELS"
)]
pub async fn change(ctx: Context<'_>, channel: Channel) -> Result<(), Error> {
    let channel = channel.id().into_db();
    // SAFETY: Since this command is guild_only this should NEVER fail
    let guild = ctx.guild_id().unwrap().into_db();

    sqlx::query!(
        "UPDATE guild SET goodbye_channel = $1 WHERE guild.discord_id = $2",
        channel,
        guild
    )
    .execute(&ctx.data().db)
    .await?;

    ctx.say("Done, make sure to have at least a single goodbye message!")
        .await?;

    Ok(())
}
