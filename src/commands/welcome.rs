use crate::serenity::Channel;
use crate::{Context, Error};

#[poise::command(prefix_command, slash_command, subcommands("message", "channel"))]
pub async fn welcome(_ctx: Context<'_>, _arg: String) -> Result<(), Error> {
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
    // Since this command is guild_only this should NEVER fail
    let guild = ctx.guild_id().unwrap().as_u64().to_be_bytes();

    sqlx::query!("UPDATE guild SET welcome_messages = array_append(welcome_messages, $1) WHERE guild.discord_id = $2", message, &guild)
        .execute(&ctx.data().db)
        .await?;

    ctx.say("Done!").await?;

    Ok(())
}

///Lists all current welcome messages
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MANAGE_CHANNELS"
)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    // Since this command is guild_only this should NEVER fail
    let guild = ctx.guild_id().unwrap().as_u64().to_be_bytes();

    let welcome_messages = sqlx::query!("SELECT welcome_messages FROM guild WHERE guild.discord_id = $1", &guild)
        .fetch_one(&ctx.data().db)
        .await?.welcome_messages;

    ctx.say(welcome_messages.join("\n")).await?;

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
    let channel = channel.id().as_u64().to_be_bytes();
    // Since this command is guild_only this should NEVER fail
    let guild = ctx.guild_id().unwrap().as_u64().to_be_bytes();

    sqlx::query!(
        "UPDATE guild SET welcome_channel = $1 WHERE guild.discord_id = $2",
        &channel,
        &guild
    )
    .execute(&ctx.data().db)
    .await?;

    ctx.say("Done, make sure to have at least a single welcome message!")
        .await?;

    Ok(())
}
