use crate::database::{self, IntoDatabase};
use crate::{serenity, Data, Error};
use poise::serenity_prelude::{CreateMessage, Mention, Mentionable};
use serenity::model::channel::MessageFlags;
use serenity::{ChannelId, Context, Member};
use std::fmt::Write;

pub async fn handle(new_member: &Member, data: &Data, ctx: &Context) -> Result<(), Error> {
    let welcome_configs = sqlx::query!(
        r#"SELECT welcome_channel as "welcome_channel: database::ChannelId", (welcome_messages)[1 + trunc(random() * array_length(welcome_messages, 1))::int] as welcome_message
                    FROM guild WHERE guild.discord_id = $1"#,
        new_member.guild_id.into_db()
    )
    .fetch_one(&data.db)
    .await?;

    let Some(welcome_channel) = welcome_configs.welcome_channel else {
        return Ok(());
    };

    membership_event(
        ctx,
        welcome_channel.into_serenity(),
        welcome_configs.welcome_message,
        "{} joined a server without any welcome message, how uncreative!",
        new_member.mention(),
    )
    .await
}

pub async fn membership_event(
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

    channel
        .send_message(
            ctx,
            CreateMessage::new()
                .content(message)
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
        )
        .await?;

    Ok(())
}
