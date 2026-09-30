// `sqlx::migrate!` embeds ./migrations at compile time, but Cargo doesn't
// know that — without this, adding a new .sql file leaves a stale binary
// running the old migration set until some .rs file changes.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
