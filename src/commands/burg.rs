use std::collections::HashMap;
use std::fmt::Write;
use std::pin::pin;
use std::{iter, ops::Not};

use futures_util::StreamExt;
use poise::serenity_prelude::{Member, Message, ReactionType, RoleId, User, UserId};

use crate::{Context, Error};

#[poise::command(prefix_command, guild_only, aliases("b", "burg_vote"))]
pub async fn burg_vote(ctx: Context<'_>, message: Message) -> Result<(), Error> {
    let mut users_votes = HashMap::<UserId, (ReactionType, Option<ReactionType>, bool)>::new();

    for reactions in &message.reactions {
        for user in message
            .reaction_users(ctx, reactions.reaction_type.clone(), None, None)
            .await?
        {
            match users_votes.get_mut(&user.id) {
                Some((_, second @ None, false)) => *second = Some(reactions.reaction_type.clone()),
                Some((_, None, true)) => {
                    unreachable!("Unexpected true on bool for multivoting when 2nd is not Some")
                }
                Some((_, Some(_), bool)) => *bool = true,
                None => {
                    users_votes.insert(user.id, (reactions.reaction_type.clone(), None, false));
                }
            }
        }
    }

    let mut party_votes = HashMap::<ReactionType, usize>::with_capacity(message.reactions.len());

    let mut multivoters = Vec::new();

    for (user, (first, second, multivote)) in users_votes.into_iter() {
        if multivote {
            multivoters.push(user);
            continue;
        }

        match second {
            Some(second) => {
                *party_votes.entry(first.clone()).or_insert(0) += 1;
                *party_votes.entry(second.clone()).or_insert(0) += 1;
            }
            None => *party_votes.entry(first.clone()).or_insert(0) += 2,
        };
    }

    let total: usize = party_votes.values().sum();

    let mut formatted = String::new();

    // Maintain reaction order
    for reaction in &message.reactions {
        let (party, votes) = party_votes.remove_entry(&reaction.reaction_type).unwrap();

        let percent = votes as f64 / total as f64;

        write!(
            &mut formatted,
            "\n {party} ({votes}): {:.2}%",
            percent * 100.0
        )?;
    }

    if multivoters.is_empty() {
        ctx.say(format!("Current Percentages (Total {total}): {formatted}"))
            .await?;
    } else {
        ctx.say(format!(
            "Current Percentages (Total {total}): {formatted}\nMultivoters: {multivoters:?}"
        ))
        .await?;
    }

    Ok(())
}

#[poise::command(prefix_command, guild_only, aliases("silly_check"))]
pub async fn silly_check(ctx: Context<'_>, message: Message) -> Result<(), Error> {
    let mut users_votes = HashMap::<User, (ReactionType, Vec<ReactionType>)>::new();

    for reactions in &message.reactions {
        for user in message
            .reaction_users(ctx, reactions.reaction_type.clone(), None, None)
            .await?
        {
            users_votes
                .entry(user)
                .and_modify(|(_, l)| l.push(reactions.reaction_type.clone()))
                .or_insert((reactions.reaction_type.clone(), Vec::new()));
        }
    }

    users_votes.retain(|_, (_, extra)| extra.is_empty().not());

    if users_votes.is_empty() {
        ctx.say("Message has no multi voters!").await?;
        return Ok(());
    }

    let mut adjusted_reacitons = message
        .reactions
        .iter()
        .cloned()
        .map(|r| (r.reaction_type, r.count))
        .collect::<HashMap<_, _>>();

    let mut final_output = String::from("Multivoters:");

    for (voter, (first, rest)) in users_votes {
        write!(&mut final_output, "\n- {} (", voter.name)?;
        for reaction in iter::once(&first).chain(&rest) {
            write!(&mut final_output, "{reaction} ")?;

            *adjusted_reacitons.get_mut(reaction).unwrap() -= 1;
        }
        final_output.pop();
        final_output.push(')');
    }
    final_output.push_str("\n\nAdjusted Reactions:");

    for reaction in message.reactions {
        let (reaction_type, adjusted) = adjusted_reacitons
            .get_key_value(&reaction.reaction_type)
            .unwrap();

        write!(&mut final_output, "\n{reaction_type}: {adjusted}")?;
    }

    ctx.say(final_output).await?;

    Ok(())
}

#[poise::command(prefix_command, guild_only, aliases("silly_give"))]
pub async fn mp_give(ctx: Context<'_>, users: Vec<Member>) -> Result<(), Error> {
    if ctx.author().id != 203353033814310912
        && ctx
            .author_member()
            .await
            .unwrap()
            .permissions(ctx)?
            .manage_roles()
            .not()
    {
        ctx.say("You are not Silly!").await?;
        return Ok(());
    }

    for mut user in users {
        user.add_role(ctx, RoleId::from(1257163650230124605))
            .await?;
    }
    ctx.say("Added!").await?;

    Ok(())
}

#[poise::command(prefix_command, guild_only, aliases("silly_destroy"))]
pub async fn mp_destroy(ctx: Context<'_>, users: Vec<Member>) -> Result<(), Error> {
    if ctx.author().id != 203353033814310912
        && ctx
            .author_member()
            .await
            .unwrap()
            .permissions(ctx)?
            .manage_roles()
            .not()
    {
        ctx.say("You are not Silly!").await?;
        return Ok(());
    }

    for mut user in users {
        user.remove_role(ctx, RoleId::from(1257163650230124605))
            .await?;
    }
    ctx.say("Removed!").await?;

    Ok(())
}

#[poise::command(prefix_command, guild_only, aliases("silly_clear"))]
pub async fn mp_clear(ctx: Context<'_>) -> Result<(), Error> {
    if ctx.author().id != 203353033814310912
        && ctx
            .author_member()
            .await
            .unwrap()
            .permissions(ctx)?
            .manage_roles()
            .not()
    {
        ctx.say("You are not Silly!").await?;
        return Ok(());
    }

    let mut member_list = pin!(ctx.guild().unwrap().id.members_iter(ctx));

    while let Some(member) = member_list.next().await {
        member?
            .remove_role(ctx, RoleId::from(1257163650230124605))
            .await?;
    }

    ctx.say("All Shot!").await?;

    Ok(())
}

#[poise::command(prefix_command, guild_only, aliases("silly_prune"))]
pub async fn mp_prune(ctx: Context<'_>, message: Message) -> Result<(), Error> {
    if ctx.author().id != 203353033814310912
        && ctx
            .author_member()
            .await
            .unwrap()
            .permissions(ctx)?
            .manage_messages()
            .not()
    {
        ctx.say("You are not Silly!").await?;
        return Ok(());
    }

    let mut pruned = String::new();

    for reaction in &message.reactions {
        for user in message
            .reaction_users(ctx, reaction.reaction_type.clone(), None, None)
            .await?
        {
            let has_mp = user
                .has_role(
                    ctx,
                    ctx.guild_id().unwrap(),
                    RoleId::from(1257163650230124605),
                )
                .await?;

            if has_mp.not() {
                message
                    .delete_reaction(ctx, Some(user.id), reaction.reaction_type.clone())
                    .await?;
                pruned.push_str("\n- ");
                pruned.push_str(&user.name);
            }
        }
    }

    ctx.say("Pruned!").await?;

    Ok(())
}
