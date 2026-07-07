// Triggers rebuild when migrations change
fn main() {
    println!("cargo:rerun-if-changed=migrations/*");
    println!("cargo:rerun-if-changed=migrations/postgres/*");
    println!("cargo:rerun-if-changed=migrations/postgres/20260708000000_saas_orgs.sql");
    println!("cargo:rerun-if-changed=migrations/postgres/20260708000000_saas_orgs.down.sql");
}