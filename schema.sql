CREATE TABLE IF NOT EXISTS discord_guild (
    `guild_id` INTEGER PRIMARY KEY AUTOINCREMENT,
    `discord_id` varchar(255) NOT NULL
    );

CREATE TABLE IF NOT EXISTS arma_servers (
    `server_id` INTEGER PRIMARY KEY AUTOINCREMENT,
    `guild_id` INTEGER NOT NULL,
    `addr` varchar(255) NOT NULL,
    FOREIGN KEY (guild_id) REFERENCES discord_guild(guild_id)
    );
