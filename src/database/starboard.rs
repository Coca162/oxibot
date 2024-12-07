use core::mem;
use std::fmt::Write;

use crate::database::{self, IntoDatabase};
use crate::{Data, Error};
use poise::serenity_prelude::{
    ChannelId, Context, CreateMessage, EditMessage, GuildId, Message, MessageId, Reaction, User,
};
use serenity::all::{HttpError, Mentionable, MessageFlags, MessageReference, StatusCode};
use sqlx::Error as SQLxError;

pub async fn add_starboard_tables(
    data: &Data,
    guild_id: GuildId,
    channel_id: ChannelId,
    emoji: &str,
    min_reactions: i32,
) -> Result<(), SQLxError> {
    sqlx::query!(
        "INSERT INTO starboard (guild_id, emoji, starboard_channel, min_reactions) VALUES ($1, $2, $3, $4)",
        guild_id.into_db(),
        emoji,
        channel_id.into_db(),
        min_reactions
    )
    .execute(&data.db)
    .await?;

    Ok(())
}

/// Manages the starboard response to a change in reactions
pub async fn manage_starboard_entry(
    ctx: &Context,
    data: &Data,
    reaction: &Reaction,
) -> Result<(), Error> {
    // Check if this reaction is in a guild, and get guild id
    let guild_id = match reaction.guild_id {
        Some(id) => id.into_db(),
        None => return Ok(()),
    };

    let emoji = reaction.emoji.clone();
    let emoji_string = emoji.to_string();

    let possible_starboard = sqlx::query!(
        r#"SELECT starboard_channel as "starboard_channel: database::ChannelId", min_reactions FROM starboard 
                    WHERE starboard.guild_id = $1 AND starboard.emoji = $2"#,
                    guild_id,
        emoji_string
    )
    .fetch_optional(&data.db)
    .await?;

    // Return if we don't have a starboard for this emoji
    let (starboard_channel, min_reactions) = match possible_starboard {
        Some(record) => (
            record.starboard_channel.into_serenity(),
            record.min_reactions,
        ),
        None => return Ok(()),
    };

    let message = reaction.message(ctx).await?;

    let reactions = message
        .reaction_users(ctx, emoji, Some(100), None)
        .await
        .unwrap_or(vec![]);

    add_or_edit_starboard_entry(
        ctx,
        data,
        message,
        &reactions,
        emoji_string.as_str(),
        starboard_channel,
        min_reactions,
    )
    .await?;

    Ok(())
}

/// Edits a starboard entry, or creates one if one does not exist
async fn add_or_edit_starboard_entry(
    ctx: &Context,
    data: &Data,
    message: Message,
    reactions: &[User],
    emoji_string: &str,
    channel: ChannelId,
    min_reactions: i32,
) -> Result<(), Error> {
    let possible_entry = sqlx::query!(
        r#"SELECT starboard_post_id as "id: database::MessageId", starboard_channel as "channel: database::ChannelId" FROM starboard_tracked 
                    WHERE starboard_tracked.message_id = $1 AND starboard_tracked.emoji = $2"#,
        message.id.into_db(),
        emoji_string
    )
    .fetch_optional(&data.db)
    .await?;

    let length = {
        if reactions.contains(&message.author) {
            reactions.len() - 1
        } else {
            reactions.len()
        }
    };

    match possible_entry {
        Some(post) => {
            edit_starboard_entry(
                ctx,
                data,
                post.id.into_serenity(),
                post.channel.into_serenity(),
                reactions.len(),
                emoji_string,
            )
            .await?
        }
        None if length >= min_reactions.try_into().unwrap() => {
            add_starboard_entry(ctx, data, message, channel, emoji_string, reactions.len()).await?
        }
        None => (),
    }

    Ok(())
}

/// Creates a new starboard entry
async fn add_starboard_entry(
    ctx: &Context,
    data: &Data,
    message: Message,
    starboard_channel: ChannelId,
    emoji_string: &str,
    current_reactions: usize,
) -> Result<(), Error> {
    let mut tx = data.db.begin().await?;

    // Add entry with temporary ID only for this transaction
    sqlx::query!(
        r#"INSERT INTO starboard_tracked 
        (message_id, emoji, starboard_channel, starboard_post_id, reaction_count) VALUES ($1, $2, $3, 666, $4)"#,
        message.id.into_db(),
        emoji_string,
        starboard_channel.into_db(),
        current_reactions as i32
    ).execute(&mut *tx)
    .await?;

    let post = starboard_channel
        .send_message(
            ctx,
            CreateMessage::new()
                .content(format!(
                    "{} | {emoji_string} {current_reactions}",
                    message.author.mention()
                ))
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
        )
        .await?;

    starboard_channel
        .send_message(
            ctx,
            CreateMessage::new().reference_message(
                MessageReference::new(
                    serenity::all::MessageReferenceKind::Forward,
                    message.channel_id,
                )
                .message_id(message.id)
                .fail_if_not_exists(true),
            ),
        )
        .await?;

    sqlx::query!(
        r#"UPDATE starboard_tracked SET starboard_post_id = $1 WHERE message_id = $2 AND emoji = $3"#,
        post.id.into_db(),
        message.id.into_db(),
        emoji_string,
    ).execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Edits an existing starboard entry
async fn edit_starboard_entry(
    ctx: &Context,
    data: &Data,
    message: MessageId,
    channel: ChannelId,
    reactions: usize,
    emoji_string: &str,
) -> Result<(), Error> {
    let mut post = match channel.message(ctx, message).await {
        Ok(post) => post,
        Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(req)))
            if req.status_code == StatusCode::NOT_FOUND =>
        {
            sqlx::query!(
                r#"DELETE FROM starboard_tracked WHERE starboard_tracked.starboard_post_id = $1"#,
                message.into_db(),
            )
            .execute(&data.db)
            .await?;
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };

    sqlx::query!(
        "UPDATE starboard_tracked SET reaction_count = $3 WHERE starboard_tracked.starboard_post_id = $1 AND starboard_tracked.emoji = $2",
        message.into_db(),
        emoji_string,
        reactions as i32
    ).execute(&data.db)
    .await?;

    let mut content = mem::take(&mut post.content);

    let index = content
        .rfind(' ')
        .expect("Message contents in the DB should have a number at the end");
    content.truncate(index + 1);

    write!(&mut content, "{reactions}")?;

    post.edit(ctx, EditMessage::new().content(content)).await?;

    Ok(())
}

/// Removes a starboard entry and associated message. Fails silently if entry does not exist.
pub async fn remove_starboard_entry(
    ctx: &Context,
    data: &Data,
    message: &MessageId,
) -> Result<(), Error> {
    // Remove + get all entries with the message id.
    let entries: Vec<_> = sqlx::query!(
        r#"DELETE FROM starboard_tracked WHERE starboard_tracked.message_id = $1
        RETURNING starboard_post_id as "starboard_post_id: database::MessageId", starboard_channel as "starboard_channel: database::ChannelId""#,
        message.into_db(),
    )
    .fetch_all(&data.db)
    .await?;

    for entry in entries {
        let message = entry.starboard_post_id.into_serenity();

        let starboard_channel = entry.starboard_channel.into_serenity();

        starboard_channel.delete_message(ctx, message).await?;
    }

    Ok(())
}

/// Remove the starboard tables associated with `channel_id`
pub async fn delete_starboard_tables(data: &Data, channel_id: ChannelId) -> Result<(), SQLxError> {
    let id = channel_id.into_db();

    let mut trans = data.db.begin().await?;

    sqlx::query!(
        "DELETE FROM starboard_tracked WHERE starboard_tracked.starboard_channel = $1",
        id
    )
    .execute(trans.as_mut())
    .await?;

    sqlx::query!(
        "DELETE FROM starboard WHERE starboard.starboard_channel = $1",
        id
    )
    .execute(trans.as_mut())
    .await?;

    trans.commit().await?;

    Ok(())
}
