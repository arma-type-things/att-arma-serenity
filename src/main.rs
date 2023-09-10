mod commands;

// Error handling / trace logging
use anyhow::anyhow;
use tracing::{error, info};

// Serenity
use serenity::{
    async_trait,
    model::{
        gateway::Ready,
        prelude::{
            GuildId,
            Interaction,
            InteractionResponseType,
        }
    },
    prelude::*,
};

// Shuttle
use shuttle_secrets::SecretStore;
// Turso
use libsql_client::client::Client as SqlClient;
use serenity::http::CacheHttp;

struct Bot {
    steam_api_key: String,
    owner_guild_id: u64,
    arma_servers: Vec<String>,
    turso_client: SqlClient,
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        let guild_id = GuildId(self.owner_guild_id);

        let commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands.create_application_command(|command| commands::ServerStatusCommand::register(command))
        })
        .await
        .unwrap();

        info!(
            "Successfully registered application commands: {:#?}",
            commands
        );
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let response_content = match command.data.name.as_str() {
                "status" => {
                    // let channel_id = command.channel_id;
                    commands::ServerStatusCommand::run(
                        &command.data.options,
                        self.steam_api_key.to_owned(),
                        self.arma_servers.to_owned(),
                        // channel_id,
                        // ctx.to_owned()
                    )
                    .await
                }
                command => unreachable!("Unknown command: {}", command),
            };

            let create_interaction_response =
                command.create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(response_content))
                });

            if let Err(why) = create_interaction_response.await {
                error!("Cannot respond to slash command: {:?}", why);
            }
        }
    }
}

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



fn check_environment(secret_store: SecretStore) -> Result<(String, String, u64, Vec<String>), anyhow::Error> {
    let mut discord_token = String::new();
    let mut steam_api_key = String::new();
    let mut owner_guild_id: u64 = 0;
    let mut arma_servers = Vec::new();

    discord_token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    steam_api_key = if let Some(steam_key) = secret_store.get("STEAM_API_KEY") {
        steam_key
    } else {
        return Err(anyhow!("'STEAM_API_KEY' was not found").into());
    };

    owner_guild_id = if let Some(guild_id) = secret_store.get("OWNER_GUILD_ID") {
        guild_id.parse().unwrap()
    } else {
        return Err(anyhow!("'OWNER_GUILD_ID' was not found").into());
    };

    arma_servers = if let Some(arma_servers) = secret_store.get("ARMA_SERVERS") {
        arma_servers.split(',').map(|s| s.to_owned()).collect()
    } else {
        return Err(anyhow!("'ARMA_SERVERS' was not found").into());
    };

    Ok((discord_token, steam_api_key, owner_guild_id, arma_servers.to_owned()))
}


