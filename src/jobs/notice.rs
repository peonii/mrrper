use std::{collections::HashMap, sync::Arc, time::Duration};

use once_cell::sync::Lazy;
use poise::serenity_prelude::{
    Builder, Cache, ChannelId, CreateEmbed, CreateEmbedFooter, CreateMessage, Embed, EmbedFooter,
    GuildId, Http, Mentionable, Role, RoleId, Timestamp,
};
use redis::Commands;
use regex::Match;
use tokio::sync::RwLock;

use crate::{librus::handlers::notices::SchoolNotice, BotError, State};

use super::JobRunnerContext;

async fn send_timetable_changes<'a>(
    http: &Arc<Http>,
    notice: &SchoolNotice,
) -> Result<(), BotError> {
    let roles = http
        .get_guild_roles(GuildId::new(930512190220435516))
        .await?;

    let mut mentions = String::from(":bangbang: **ZMIANY W PLANIE**\n\n");

    let content = notice.content.clone();
    let content = content
        .lines()
        .map(|l| {
            let role_regex = regex::Regex::new(r"[1-4][a-zA-Z][a-zA-Z]?( )?-").expect("oops");
            let mut roles_matched = vec![];
            let mut count = 1;
            let mut offset = 0;

            for cap in role_regex.captures_iter(l) {
                let role_str = cap.get(0).unwrap().as_str();
                let role_num = role_str.chars().nth(0).unwrap();
                let possible_role_letters = vec![
                    role_str.chars().nth(1).unwrap(),
                    role_str.chars().nth(2).unwrap(),
                ];
                dbg!(&possible_role_letters);
                let mut role = roles
                    .iter()
                    .filter(|r| {
                        r.name.to_lowercase().contains(role_num)
                            && possible_role_letters
                                .iter()
                                .any(|l| r.name.to_lowercase().contains(*l))
                            && r.name.len() == 2
                    })
                    .collect::<Vec<&Role>>();

                roles_matched.append(&mut role);
                count += 1;
            }

            let mut indexes = vec![];

            for role in roles_matched {
                indexes.push((offset, role));
                offset += role.mention().to_string().len();
            }

            let mut new_line = l.to_string();

            for (idx, role) in indexes {
                let mention = role.mention();
                new_line.replace_range(idx..idx + count, &mention.to_string());
                count = 0;

                mentions.push_str(&mention.to_string());
            }

            new_line
        })
        .collect::<Vec<String>>()
        .join("\n");

    let embed = CreateEmbed::new()
        .title(notice.title.clone())
        .description(content)
        .footer(CreateEmbedFooter::new(notice.created_at.clone()));

    let msg = CreateMessage::new().content(mentions).add_embed(embed);

    if let Err(e) = msg
        .execute(
            http.as_ref(),
            (
                ChannelId::new(930552545179500636),
                Some(GuildId::new(930512190220435516)),
            ),
        )
        .await
    {
        tracing::error!("An error occurred when sending notice: {e}");
    }

    Ok(())
}

async fn send_notice<'a>(_cache: &Arc<Cache>, http: &Arc<Http>, notice: &SchoolNotice) {
    if notice.title.to_lowercase().contains("zmiany w") {
        send_timetable_changes(http, notice).await;
        return;
    }

    let mut content = notice.content.clone();
    content.truncate(2048);
    let embed = CreateEmbed::new()
        .title(notice.title.clone())
        .description(content)
        .footer(CreateEmbedFooter::new(notice.created_at.clone()));

    let msg = CreateMessage::new()
        .content(":bangbang: **Nowe Ogłoszenie**")
        .add_embed(embed);

    if let Err(e) = msg
        .execute(
            http.as_ref(),
            (
                ChannelId::new(1283505729592103025),
                Some(GuildId::new(930512190220435516)),
            ),
        )
        .await
    {
        tracing::error!("An error occurred when sending notice: {e}");
    }
}

pub async fn notice_runner<'a>(ctx: &JobRunnerContext<State<'a>>) -> Result<(), BotError> {
    let notices = ctx.state.librus.fetch_notices().await?;

    for notice in notices.iter() {
        let digest = ring::digest::digest(&ring::digest::SHA256, notice.content.as_bytes());
        let bytes = digest.as_ref();
        let digest: String = String::from_utf8_lossy(bytes).into();

        let mut conn = ctx.state.redis.get_connection()?;
        let cached: Option<String> = conn.get(&notice.id)?;

        if let Some(cached) = cached {
            if cached.clone() != digest {
                conn.set(&notice.id, digest)?;
                send_notice(&ctx.cache, &ctx.http, notice).await;
            }
        } else {
            conn.set(&notice.id, digest)?;
            send_notice(&ctx.cache, &ctx.http, notice).await;
        }
    }

    tokio::time::sleep(Duration::from_secs(4 * 60)).await;

    Ok(())
}
