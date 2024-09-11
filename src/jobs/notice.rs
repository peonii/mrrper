use std::{collections::HashMap, sync::Arc, time::Duration};

use once_cell::sync::Lazy;
use poise::serenity_prelude::{
    Builder, Cache, ChannelId, CreateEmbed, CreateEmbedFooter, CreateMessage, Embed, EmbedFooter,
    GuildId, Http, Timestamp,
};
use tokio::sync::RwLock;

use crate::{librus::handlers::notices::SchoolNotice, State};

use super::JobRunnerContext;

pub struct NoticeCache {
    pub cache: HashMap<String, String>,
}

impl Default for NoticeCache {
    fn default() -> Self {
        Self {
            cache: HashMap::default(),
        }
    }
}

pub static NOTICE_CACHE: Lazy<RwLock<NoticeCache>> =
    Lazy::new(|| RwLock::new(NoticeCache::default()));

async fn send_notice<'a>(cache: &Arc<Cache>, http: &Arc<Http>, notice: &SchoolNotice) {
    let mut content = notice.content.clone();
    content.truncate(2048);
    let embed = CreateEmbed::new()
        .title(notice.title.clone())
        .description(content)
        .footer(CreateEmbedFooter::new(notice.created_at.clone()));

    let msg = CreateMessage::new()
        .content(":bangbang: **Nowe Ogłoszenie**")
        .add_embed(embed);

    {
        msg.execute(
            http.as_ref(),
            (
                ChannelId::new(930552545179500636),
                Some(GuildId::new(930512190220435516)),
            ),
        )
        .await;
        //chan.send_message(http.as_ref(), msg).await;
    }
}

pub async fn notice_runner<'a>(ctx: &JobRunnerContext<State<'a>>) {
    {
        //TODO: Make this better LMFAO
        let mut cache = NOTICE_CACHE.write().await;
        let cache_json = std::fs::read("notices.json").expect("oops");
        cache.cache = serde_json::from_slice(cache_json.as_slice()).unwrap();
    }

    loop {
        let notices = ctx.state.librus.fetch_notices().await;

        if let Ok(notices) = notices {
            for notice in notices.iter() {
                let digest = ring::digest::digest(&ring::digest::SHA256, notice.content.as_bytes());
                let bytes = digest.as_ref();
                let digest: String = String::from_utf8_lossy(bytes).into();

                let cache = NOTICE_CACHE.read().await;
                let cached = cache.cache.get(&notice.id).cloned();
                drop(cache);

                if let Some(cached) = cached {
                    if *cached.clone() == digest {
                        println!("Hit cache!");
                        continue;
                    } else {
                        println!("Hit cache but stale!");
                        let mut cache = NOTICE_CACHE.write().await;
                        cache.cache.insert(notice.id.clone(), digest.clone());
                        drop(cache);
                        send_notice(&ctx.cache, &ctx.http, notice).await;
                    }
                } else {
                    println!("Didn't hit cache!");
                    let mut cache = NOTICE_CACHE.write().await;
                    cache.cache.insert(notice.id.clone(), digest.clone());
                    drop(cache);
                    send_notice(&ctx.cache, &ctx.http, notice).await;
                }
            }
        }

        println!("Dumping notices");

        let nc = NOTICE_CACHE.read().await;
        let c = serde_json::to_string(&nc.cache).unwrap();
        std::fs::write("notices.json", c);
        drop(nc);

        tokio::time::sleep(Duration::from_secs(4 * 60)).await;
    }
}
