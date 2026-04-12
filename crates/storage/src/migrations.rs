pub const INIT_SQL: &str = include_str!("migrations/init.sql");
pub const MIGRATION_002: &str = include_str!("migrations/002_users.sql");
pub const MIGRATION_003: &str = include_str!("migrations/003_refresh_tokens.sql");
pub const MIGRATION_004: &str = include_str!("migrations/004_channels.sql");

pub struct Migration {
    pub version: &'static str,
    pub sql: &'static str,
}

pub const ALL_MIGRATIONS: &[Migration] = &[
    Migration { version: "001_init", sql: INIT_SQL },
    Migration { version: "002_users", sql: MIGRATION_002 },
    Migration { version: "003_refresh_tokens", sql: MIGRATION_003 },
    Migration { version: "004_channels", sql: MIGRATION_004 },
];
