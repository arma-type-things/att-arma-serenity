mod commands;
mod bot;

// Serenity
use serenity::prelude::*;

use libsql_client::Client as SqlClient;
use shuttle_secrets::SecretStore;

use bot::{Bot, check_environment};

#[shuttle_runtime::main]
async fn att_bot(
    #[shuttle_turso::Turso(addr = "{secrets.DB_TURSO_ADDR}", token = "{secrets.DB_TURSO_TOKEN}")]
    turso_client: SqlClient,
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {

    // TODO: Move arma servers to database call using guild_id
    let (
        discord_token,
        steam_api_key,
        owner_guild_id,
        arma_servers) = check_environment(secret_store).unwrap();

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_SCHEDULED_EVENTS
        | GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(&discord_token, intents)
        .event_handler(Bot {
            steam_api_key,
            owner_guild_id,
            arma_servers,
            turso_client,
        })
        .await
        .expect("Err creating client");

    Ok(client.into())
}


