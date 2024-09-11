use mrrper::BotError;

#[tokio::main]
async fn main() -> Result<(), BotError> {
    dotenvy::dotenv().ok();

    mrrper::start(&std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not provided")).await?;

    Ok(())
}
