use std::sync::Arc;

use jobs::JobRunner;
use librus::{
    client::{LibrusClient, LibrusCredentials},
    handlers::notices,
};
use poise::serenity_prelude;
use thiserror::Error;

use poise::serenity_prelude as serenity;

pub mod commands;
pub mod jobs;
pub mod librus;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("An error has occurred with the bot.")]
    SerenityError(#[from] serenity_prelude::Error),

    #[error("An error has occurred with Librus.")]
    LibrusError(#[from] crate::librus::client::LibrusError),

    #[error("An error has occurred with Redis.")]
    RedisError(#[from] redis::RedisError),
}

#[derive(Clone)]
pub struct State<'a> {
    pub librus: Arc<LibrusClient<'a>>,
    pub redis: redis::Client,
}

pub async fn start(token: &str) -> Result<(), BotError> {
    let mut librus = LibrusClient::new()?.with_credentials(LibrusCredentials {
        email: std::env::var("LIBRUS_EMAIL")
            .expect("LIBRUS_EMAIL not found")
            .into(),
        password: std::env::var("LIBRUS_PASSWORD")
            .expect("LIBRUS_PASSWORD not found")
            .into(),
    });

    let rdc = redis::Client::open(std::env::var("REDIS_URL").expect("REDIS_URL not found"))?;

    librus.login().await?;

    let state = State {
        librus: Arc::new(librus),
        redis: rdc,
    };

    let job_state = state.clone();

    let framework: poise::Framework<State, BotError> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(state)
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    let mut job_runner = JobRunner::new(&client, job_state);
    job_runner.start(jobs::notice::notice_runner).await;

    client.start().await?;

    job_runner.stop().await;

    Ok(())
}
