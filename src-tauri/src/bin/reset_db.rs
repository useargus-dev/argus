//! Dev utility: delete local Argus data (~/.argus by default).
//!
//! ```bash
//! cargo run --bin reset_db
//! ARGUS_DATA_DIR=/tmp/argus-test cargo run --bin reset_db
//! ```

use argus_lib::db::{argus_dir, reset_local_data};

fn main() {
    let dir = argus_dir();
    eprintln!("Argus data directory: {}", dir.display());

    match reset_local_data() {
        Ok(false) => {
            eprintln!("Nothing to remove (directory does not exist).");
        }
        Ok(true) => {
            eprintln!("Removed {}", dir.display());
            eprintln!("Reset complete. Restart the app to register again.");
        }
        Err(e) => {
            eprintln!("Reset failed: {e}");
            std::process::exit(1);
        }
    }
}
