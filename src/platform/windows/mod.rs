//! Windows backend.

mod autostart;
mod elevation;
mod hook;
mod instance;
mod resizer;
mod tray;
mod util;
mod window;

use std::time::Duration;

use crate::shared::settings;
use instance::InstanceLock;

/// Launch flag: take over from a copy that's already running (e.g. after an update).
const REPLACE_ARG: &str = "--replace";

pub fn run() {
    let elevated = elevation::is_elevated();
    let relaunched = std::env::args().any(|a| a == elevation::ELEVATED_ARG);
    let replace = std::env::args().any(|a| a == REPLACE_ARG);

    // After an elevated relaunch, the old copy needs a moment to exit.
    let wait = if relaunched { Duration::from_secs(5) } else { Duration::ZERO };
    let lock = InstanceLock::acquire(wait).or_else(|| {
        replace.then(|| {
            instance::ask_existing_to_quit();
            InstanceLock::acquire(Duration::from_secs(5))
        })?
    });
    let Some(mut lock) = lock else {
        instance::signal_existing();
        return;
    };

    // The user chose "Restart as administrator" before: hand over to an elevated copy.
    if settings::current().run_as_admin && !elevated && !relaunched {
        drop(lock);
        if elevation::launch_elevated() {
            return;
        }
        // UAC declined: carry on without admin.
        let Some(l) = InstanceLock::acquire(Duration::from_secs(1)) else { return };
        lock = l;
    }

    autostart::sync(settings::current().run_as_admin, elevated);
    hook::start(resizer::spawn());
    tray::run(lock, elevated);
}
