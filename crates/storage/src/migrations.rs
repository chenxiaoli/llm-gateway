pub const INIT_SQL: &str = include_str!("migrations/init.sql");
pub const MIGRATION_002: &str = include_str!("migrations/002_users.sql");
pub const MIGRATION_003: &str = include_str!("migrations/003_refresh_tokens.sql");
pub const MIGRATION_004: &str = include_str!("migrations/004_channels.sql");

pub const ALL_MIGRATIONS: &[&str] = &[INIT_SQL, MIGRATION_002, MIGRATION_003, MIGRATION_004];
