use crate::database::{self, IntoDatabase};
use crate::{serenity, Data, Error};
use poise::serenity_prelude::{GuildId, Mentionable, User};
use serenity::Context;

pub async fn handle(
    guild_id: &GuildId,
    user: &User,
    data: &Data,
    ctx: &Context,
) -> Result<(), Error> {
    let goodbye_configs = sqlx::query!(
        r#"SELECT goodbye_channel as "goodbye_channel: database::ChannelId", (goodbye_messages)[1 + trunc(random() * array_length(goodbye_messages, 1))::int] as goodbye_message
                    FROM guild WHERE guild.discord_id = $1"#,
        guild_id.into_db()
    )
    .fetch_one(&data.db)
    .await?;

    let Some(goodbye_channel) = goodbye_configs.goodbye_channel else {
        return Ok(());
    };

    // TODO: Fucking figure out a nice DRY way to do this
    // that doesn't mean we have to keep shared stuff in one or the other
    super::guild_member_addition::membership_event(
        ctx,
        goodbye_channel.into_serenity(),
        goodbye_configs.goodbye_message,
        "{} left a server without any goodbye message, how uncreative!",
        user.mention(),
    )
    .await
}
