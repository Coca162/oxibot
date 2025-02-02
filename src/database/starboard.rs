use core::mem;

use crate::database::{self, IntoDatabase};
use crate::{Data, Error, EMBED_COLOR};
use poise::serenity_prelude::{
    ChannelId, Context, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage,
    EditMessage, GuildId, Message, MessageId, Reaction, User,
};
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

    let length = {
        if reactions.contains(&message.author) {
            reactions.len() - 1
        } else {
            reactions.len()
        }
    };

    if length >= min_reactions.try_into().unwrap() {
        add_or_edit_starboard_entry(
            ctx,
            data,
            message,
            &reactions,
            emoji_string.as_str(),
            starboard_channel,
        )
        .await?;
    } else {
        remove_starboard_entry_with_channel(ctx, data, message.id, starboard_channel).await?;
    }

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
) -> Result<(), Error> {
    let possible_entry = sqlx::query!(
        r#"SELECT starboard_post_id as "id: database::MessageId", starboard_channel as "channel: database::ChannelId" FROM starboard_tracked 
                    WHERE starboard_tracked.message_id = $1 AND starboard_tracked.emoji = $2"#,
        message.id.into_db(),
        emoji_string
    )
    .fetch_optional(&data.db)
    .await?;

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
        None => {
            add_starboard_entry(ctx, data, message, channel, emoji_string, reactions.len()).await?
        }
    }

    Ok(())
}

/// Creates a new starboard entry
async fn add_starboard_entry(
    ctx: &Context,
    data: &Data,
    mut message: Message,
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

    let post = {
        let mut attachments = mem::take(&mut message.attachments);

        attachments.retain(|a| {
            a.content_type
                .as_deref()
                .is_some_and(|t| t.starts_with("image/"))
        });

        let mut main_embed = CreateEmbed::new()
            .author(
                CreateEmbedAuthor::new(message.author.name.clone()).icon_url(message.author.face()),
            )
            .url("http://example.com/0")
            .description(message.content.clone())
            .color(EMBED_COLOR);

        if let Some(message) = &message.referenced_message {
            main_embed = main_embed.field("Replied Message:", &message.content, false);
        }

        let mut extra_embeds = Vec::new();

        if let Some((first, extra)) = attachments.split_first() {
            main_embed = main_embed.image(first.url.as_str());

            let mut i = 1;
            for attachment in extra[..extra.len().min(10)].iter() {
                let embed = CreateEmbed::new()
                    .color(EMBED_COLOR)
                    .url(format!("http://example.com/{}", i / 4))
                    .image(attachment.url.as_str());

                extra_embeds.push(embed);
                i += 1;
            }
        }

        let last = extra_embeds.last_mut().unwrap_or(&mut main_embed);
        *last = mem::take(last)
            .footer(CreateEmbedFooter::new(message.id.to_string()))
            .timestamp(message.timestamp);

        CreateMessage::new()
            .content(format!(
                "{} | {emoji_string} {current_reactions}",
                message.link()
            ))
            .add_embed(main_embed)
            .add_embeds(extra_embeds)
    };

    let post = starboard_channel.send_message(ctx, post).await?;

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
    let mut post = channel.message(ctx, message).await?;

    sqlx::query!(
        "UPDATE starboard_tracked SET reaction_count = $3 WHERE starboard_tracked.starboard_post_id = $1 AND starboard_tracked.emoji = $2",
        message.into_db(),
        emoji_string,
        reactions as i32
    ).execute(&data.db)
    .await?;

    let content =
        post.content.trim_end_matches(char::is_numeric).to_string() + &reactions.to_string();

    post.edit(ctx, EditMessage::new().content(content)).await?;

    Ok(())
}

/// Removes a starboard entry and associated message in provided channel. Fails silently if entry does not exist.
pub async fn remove_starboard_entry_with_channel(
    ctx: &Context,
    data: &Data,
    message: MessageId,
    starboard_channel: ChannelId,
) -> Result<(), Error> {
    let records = sqlx::query!(
        r#"DELETE FROM starboard_tracked WHERE starboard_tracked.message_id = $1 AND starboard_tracked.starboard_channel = $2
        RETURNING starboard_post_id as "starboard_post_id: database::MessageId""#,
        message.into_db(),
        starboard_channel.into_db()
    )
    .fetch_all(&data.db)
    .await?;

    for record in records {
        let message = record.starboard_post_id.into_serenity();

        starboard_channel.delete_message(ctx, message).await?;
    }

    Ok(())
}

/// Removes a starboard entry and associated message. Fails silently if entry does not exist.
pub async fn remove_starboard_entry(
    ctx: &Context,
    data: &Data,
    message: &MessageId,
) -> Result<(), Error> {
    // Remove + get all entries with the message id. This should return a vec of length zero or one, but is not guaranteed
    let entries: Vec<_> = sqlx::query!(
        r#"DELETE FROM starboard_tracked WHERE starboard_tracked.message_id = $1
        RETURNING starboard_post_id as "starboard_post_id: database::MessageId", starboard_channel as "starboard_channel: database::ChannelId""#,
        message.into_db(),
    )
    .fetch_all(&data.db)
    .await?;

    // If there are duplicate entries, delete all of them
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
