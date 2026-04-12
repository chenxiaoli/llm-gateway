pub const INIT_SQL: &str = include_str!("migrations/init.sql");
pub const MIGRATION_002: &str = include_str!("migrations/002_users.sql");

pub const ALL_MIGRATIONS: &[&str] = &[INIT_SQL, MIGRATION_002];
