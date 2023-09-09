use anyhow::anyhow;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::prelude::{GuildId, Interaction, InteractionResponseType};
use serenity::prelude::*;
use shuttle_secrets::SecretStore;
use std::time::Duration;
use tracing::{error, info};

use serde::Deserialize;
use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::application_command::CommandDataOption;
use serenity::utils::MessageBuilder;
use tokio::time::sleep;

struct Bot {
    steam_api_key: String,
    owner_guild_id: u64,
    arma_servers: Vec<String>,
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        let guild_id = GuildId(self.owner_guild_id);

        let commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands.create_application_command(|command| ServerStatusCommand::register(command))
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
                    ServerStatusCommand::run(
                        &command.data.options,
                        self.steam_api_key.to_owned(),
                        self.arma_servers.to_owned(),
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
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let discord_token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    let steam_api_key = if let Some(steam_key) = secret_store.get("STEAM_API_KEY") {
        steam_key
    } else {
        return Err(anyhow!("'STEAM_API_KEY' was not found").into());
    };

    let owner_guild_id: u64 = if let Some(guild_id) = secret_store.get("OWNER_GUILD_ID") {
        guild_id.parse().unwrap()
    } else {
        return Err(anyhow!("'OWNER_GUILD_ID' was not found").into());
    };

    let arma_servers: Vec<String> = if let Some(arma_servers) = secret_store.get("ARMA_SERVERS") {
        arma_servers.split(',').map(|s| s.to_owned()).collect()
    } else {
        return Err(anyhow!("'ARMA_SERVERS' was not found").into());
    };

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(&discord_token, intents)
        .event_handler(Bot {
            steam_api_key,
            owner_guild_id,
            arma_servers,
        })
        .await
        .expect("Err creating client");

    Ok(client.into())
}

pub struct ServerStatusCommand;

impl ServerStatusCommand {
    pub async fn run(
        _options: &[CommandDataOption],
        steam_key: String,
        servers: Vec<String>,
    ) -> String {
        let mut response = MessageBuilder::new();

        response.push_line("Sure!");

        for server in servers {
            match Self::fetch_steam_data(&steam_key, &server).await {
                Ok(steam_response) => {
                    if let Some(servers) = steam_response.response.servers {
                        if let Some(server) = servers.first() {
                            Self::push_server_details(&mut response, server);
                        };
                    } else {
                        response.push_line(format!(
                            "no server found at {} or the server is down, sorry!",
                            &server
                        ));
                    };
                }
                Err(why) => {
                    response.push_line(format!("Error grabbing details for {}: {}", server, why));
                }
            };
            let _ = sleep(Duration::from_millis(50));
        }
        response.build()
    }
    pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        command
            .name("status")
            .description("Query the server to list all running instances.")
    }
    async fn fetch_steam_data(
        api_key: &str,
        server_details: &str,
    ) -> Result<SteamResponse<GetServerListResponse>, reqwest::Error> {
        reqwest::get(
            format!(
                "https://api.steampowered.com/IGameServersService/GetServerList/v1?key={}&filter=addr\\{}",
                api_key,
                server_details))
            .await?
            .json::<SteamResponse<GetServerListResponse>>().await
    }

    fn push_server_details(response: &mut MessageBuilder, server: &SteamServer) {
        info!("Pushing response server details for: {:?}", server);

        response.push_bold_line(format!("Server Status for {}:", server.name));

        // TODO: Fix "map not found" bug
        response
            .push_bold("Map: ")
            .push_line(format!("{}", server.map));

        response
            .push_bold("Players: ")
            .push_line(format!("{}/{}", server.players, server.max_players))
            .push_bold("Connect: ");
        let address = server.addr.split(":").collect::<Vec<&str>>();
        response.push_line(format!(
            "steam://connect/{}:{}",
            address[0], server.gameport
        ));
    }
}

// Steam structures
/// SteamResponse is a wrapper around the actual response, representing what Steam Web API returns.
#[derive(Deserialize, Debug)]
pub struct SteamResponse<T: SteamApiResponse> {
    pub response: T,
}

/// SteamApiResponse is an empty trait representing the response data from the Steam Web API.
pub trait SteamApiResponse {}

/// GetServerListResponse is the actual response data generated when calling the Steam Web API's GetServerList endpoint.
/// It contains a list of servers.
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct GetServerListResponse {
    servers: Option<Vec<SteamServer>>,
}

impl SteamApiResponse for GetServerListResponse {}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct SteamServer {
    pub addr: String,
    pub gameport: u32,
    pub steamid: String,
    pub name: String,
    pub appid: u32,
    pub gamedir: String,
    pub version: String,
    pub product: String,
    pub region: i32,
    pub players: u32,
    pub max_players: u32,
    pub bots: u32,
    pub map: String,
    pub secure: bool,
    pub dedicated: bool,
    pub os: String,
    pub gametype: String,
}

/// GetServersAtAddressResponse is the actual response data generated when calling the Steam Web API's GetServersAtAddress endpoint.
/// it has two fields, success, which is a boolean indicating whether the request was successful, and servers, which is a vector of servers.
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(crate) struct GetServersAtAddressResponse {
    pub(crate) success: bool,
    pub(crate) servers: Option<Vec<ServersAtAddress>>,
}

impl SteamApiResponse for GetServersAtAddressResponse {}

/// ServersAtAddress represents a single server as returned by the Steam Web API's GetServersAtAddress endpoint.
/// It has multiple fields, including the server's IP address, port, and game ID.
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(crate) struct ServersAtAddress {
    pub(crate) addr: String,
    pub(crate) gmsindex: i32,
    pub(crate) steamid: String,
    pub(crate) appid: i32,
    pub(crate) gamedir: String,
    pub(crate) region: i32,
    pub(crate) secure: bool,
    pub(crate) lan: bool,
    pub(crate) gameport: i32,
    pub(crate) specport: i32,
}
