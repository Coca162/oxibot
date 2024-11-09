use crate::{serenity, Data, Error};
use serenity::{Context, FullEvent};

mod channel_delete;
mod guild_member_addition;
mod guild_member_removal;
mod message_delete;
mod reaction_add;
mod reaction_remove;

pub async fn event_handler(ctx: &Context, event: &FullEvent, data: &Data) -> Result<(), Error> {
    match event {
        FullEvent::ReactionAdd { add_reaction } => {
            reaction_add::handle(add_reaction, data, ctx).await?;
        }
        FullEvent::ReactionRemove { removed_reaction } => {
            reaction_remove::handle(removed_reaction, data, ctx).await?
        }
        FullEvent::MessageDelete {
            deleted_message_id, ..
        } => {
            message_delete::handle(deleted_message_id, data, ctx).await?;
        }
        FullEvent::GuildMemberAddition { new_member } => {
            guild_member_addition::handle(new_member, data, ctx).await?;
        }
        FullEvent::GuildMemberRemoval { guild_id, user, .. } => {
            guild_member_removal::handle(guild_id, user, data, ctx).await?;
        }
        FullEvent::ChannelDelete { channel, .. } => {
            channel_delete::handle(channel, data).await?;
        }
        _ => (),
    }

    Ok(())
}
