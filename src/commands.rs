// Error handling / trace logging
use tracing::{error, info};

// Serenity
use serenity::{
    builder::CreateApplicationCommand, model::prelude::application_command::CommandDataOption,
    prelude::*, utils::MessageBuilder,
};

// Utility
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

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
