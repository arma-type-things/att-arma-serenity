// Serenity imports
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::prelude::{GuildId, ChannelId, Interaction, InteractionResponseType, ScheduledEventId, RichInvite};
use serenity::model::guild::ScheduledEvent;
use serenity::async_trait;
// use serenity::http::CacheHttp;

// Error and logging imports
use tracing::{error, info};
use anyhow::anyhow;

// Service imports
use shuttle_secrets::SecretStore;
use libsql_client::Client as SqlClient;

// Local imports
use crate::commands;

pub struct Bot {
    pub steam_api_key: String,
    pub owner_guild_id: u64,
    pub arma_servers: Vec<String>,
    pub turso_client: SqlClient,
}

// TODO: make these fetched from the database
// const TEST_CHANNEL_ID: u64 = 1021230323247349902;
const EVENT_FEED_ID: u64 = 1151219763549327471;

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

    // TODO: use real channel id
    async fn guild_scheduled_event_create(&self, ctx: Context, event: ScheduledEvent) {
        info!("Event created: {:?}", event);

        let channel = match ctx.cache.guild_channel(EVENT_FEED_ID) {
            Some(channel) => channel,
            None => {
                error!("Error creating invite, could not access channel!");
                return;
            },
        };

        info!("Channel: {:?}", channel);

        let guild_event = GuildEvent::from(event);

        info!("Guild event: {:?}", guild_event);

        let event_url = guild_event.url(&ctx).await;

        info!("Event url: {}", event_url);

        let _ = channel.say(&ctx, format!("Event Posted! {}", event_url)).await;

    }
}

async fn get_rich_invite_from_guild_event(_event: &GuildEvent, ctx: &Context) -> Result<RichInvite, anyhow::Error> {

    let channel = match ctx.cache.guild_channel(EVENT_FEED_ID) {
        Some(channel) => channel,
        None => {
            return Err(anyhow!("Error creating invite, could not access channel!"));
        },
    };

    let creation = channel
        // 14 days in seconds
        .create_invite(&ctx, |i| i.max_age(1209600 / 14).max_uses(50)).await;

    let invite = match creation {
        Ok(invite) => invite,
        Err(why) => {
            error!("Err creating invite: {:?}", why);
            return Err(anyhow!("Error creating invite: {:?}", why));
        }
    };

    Ok(invite)
}

#[derive(Debug)]
struct GuildEvent {
    // scheduled_event_id of the event
    id: ScheduledEventId,
    // guild_id of the guild hosting the event
    // guild_id: GuildId,
    // channel_id of the event (if any)
    // channel_id: Option<ChannelId>,
}

impl GuildEvent {
    fn new(id: ScheduledEventId, _guild_id: GuildId, _channel_id: Option<ChannelId>) -> Self {
        Self {
            id,
            // guild_id,
            // channel_id,
        }
    }

    pub async fn url(&self, ctx: &Context) -> String {
        let invite = get_rich_invite_from_guild_event(self, ctx).await.unwrap();
        format!("{}?event={}", invite.url(), self.id.0)
    }
}

impl From<ScheduledEvent> for GuildEvent {
    fn from(event: ScheduledEvent) -> Self {
        Self::new(event.id, event.guild_id, event.channel_id)
    }
}

pub fn check_environment(secret_store: SecretStore) -> Result<(String, String, u64, Vec<String>), anyhow::Error> {
    let discord_token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found"));
    };

    let steam_api_key = if let Some(steam_key) = secret_store.get("STEAM_API_KEY") {
        steam_key
    } else {
        return Err(anyhow!("'STEAM_API_KEY' was not found"));
    };

    let owner_guild_id = if let Some(guild_id) = secret_store.get("OWNER_GUILD_ID") {
        guild_id.parse().unwrap()
    } else {
        return Err(anyhow!("'OWNER_GUILD_ID' was not found"));
    };

    let arma_servers: Vec<String> = if let Some(arma_servers) = secret_store.get("ARMA_SERVERS") {
        arma_servers.split(',').map(|s| s.to_owned()).collect()
    } else {
        return Err(anyhow!("'ARMA_SERVERS' was not found"));
    };

    Ok((discord_token, steam_api_key, owner_guild_id, arma_servers.to_owned()))
}
