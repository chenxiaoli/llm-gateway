// Triggers rebuild when migrations change
fn main() {
    println!("cargo:rerun-if-changed=migrations/*");
    println!("cargo:rerun-if-changed=migrations/postgres/*");
    println!("cargo:rerun-if-changed=migrations/postgres/20260507000000_add_request_id.sql");
}