use std::mem;

use crate::database;
use crate::{serenity, Data, Error};
use poise::serenity_prelude::{GuildId, Mention, Mentionable, User};
use serenity::model::channel::MessageFlags;
use serenity::{ChannelId, Context};
use std::fmt::Write;

pub async fn handle(
    guild_id: &GuildId,
    user: &User,
    data: &Data,
    ctx: &Context,
) -> Result<(), Error> {
    let channel = guild_id.0 as i64;

    let goodbye_configs = sqlx::query!(
        r#"SELECT goodbye_channel as "goodbye_channel: database::ChannelId", (goodbye_messages)[1 + trunc(random() * array_length(goodbye_messages, 1))::int] as goodbye_message
                    FROM guild WHERE guild.discord_id = $1"#,
        &channel
    )
    .fetch_one(&data.db)
    .await?;

    let goodbye_channel = match goodbye_configs.goodbye_channel {
        Some(goodbye_channel) => goodbye_channel.into_serenity(),
        None => return Ok(()),
    };

    membership_event(
        ctx,
        goodbye_channel,
        goodbye_configs.goodbye_message,
        " left a server without any goodbye message, how uncreative!",
        user.mention(),
    )
    .await
}

async fn membership_event(
    ctx: &Context,
    channel: ChannelId,
    message: Option<String>,
    default_message_template: &'static str,
    user: Mention,
) -> Result<(), Error> {
    let message = message
        .map(|x| x.replace("{}", user.to_string().as_str()))
        .unwrap_or_else(|| {
            const MAX_MENTION_LEN: usize = "<@18446744073709551615>".len();
            let mut default_message =
                String::with_capacity(MAX_MENTION_LEN + default_message_template.len());
            write!(&mut default_message, "{}", user.mention()).unwrap();
            default_message.push_str(default_message_template);

            default_message
        });

    // SAFETY: we are transmuting to a u64 bitfield, and discord supports silent pings with this one
    const SILENT_FLAG: MessageFlags = unsafe { mem::transmute(4096_u64) };

    channel
        .send_message(ctx, |x| x.content(message).flags(SILENT_FLAG))
        .await?;

    Ok(())
}
