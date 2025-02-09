use std::{collections::HashMap, fmt::Write, iter, ops::Not};

use poise::serenity_prelude::{Message, ReactionType, User, UserId};

use crate::{Context, Error};

#[poise::command(prefix_command, guild_only, aliases("b", "burg_vote"))]
pub async fn burg_vote(ctx: Context<'_>, message: Message) -> Result<(), Error> {
    let mut users_votes =
        HashMap::<UserId, Result<(ReactionType, Option<ReactionType>), User>>::new();

    for reactions in &message.reactions {
        for user in message
            .reaction_users(ctx, reactions.reaction_type.clone(), None, None)
            .await?
        {
            users_votes
                .entry(user.id)
                .and_modify(|x| match x {
                    Ok((_, second @ None)) => *second = Some(reactions.reaction_type.clone()),
                    ballot @ Ok((_, Some(_))) => *ballot = Err(user),
                    Err(_) => (),
                })
                .or_insert_with(|| Ok((reactions.reaction_type.clone(), None)));
        }
    }

    let mut party_votes = HashMap::<ReactionType, usize>::with_capacity(message.reactions.len());

    let mut multivoters = Vec::new();

    for vote in users_votes.into_values() {
        match vote {
            Ok((first, Some(second))) => {
                *party_votes.entry(first).or_insert(0) += 1;
                *party_votes.entry(second).or_insert(0) += 1;
            }
            Ok((first, None)) => *party_votes.entry(first).or_insert(0) += 2,
            Err(user) => multivoters.push(user),
        }
    }

    let total: usize = party_votes.values().sum();

    let mut content = format!("Current Percentages (Total {total}):");

    // Maintain reaction order
    for party in message.reactions.iter().map(|r| &r.reaction_type) {
        let votes = party_votes.remove(party).unwrap_or(0);

        let percent = votes as f64 / total as f64;

        write!(
            &mut content,
            "\n {party} ({votes}): {:.2}%",
            percent * 100.0
        )?;
    }

    if multivoters.is_empty().not() {
        content.push_str("\n\nMultivoters:");

        for name in multivoters.iter().map(|x| x.display_name()) {
            content.push('\n');
            content.push_str(name);
        }
    }

    ctx.say(content).await?;

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
