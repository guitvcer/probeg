//! Taking readings. Sources are independent and queried in parallel, so
//! the report takes as long as the slowest one rather than the sum of them all.

mod activity;
mod apps;
mod battery;
mod hardware;
mod history;
mod memory;
mod network;
mod power;
mod smart;
mod storage;
mod usage;
mod users;

use chrono::Local;

use crate::model::{Household, Report};

/// Takes all readings.
pub fn collect() -> Report {
    std::thread::scope(|s| {
        let identity = s.spawn(hardware::identity);
        let history = s.spawn(history::timeline);
        let power = s.spawn(power::power);
        let ssd = s.spawn(smart::read);
        let disk = s.spawn(storage::disk);
        let memory = s.spawn(memory::memory);
        let battery = s.spawn(battery::battery);
        let network = s.spawn(network::network);
        let usage = s.spawn(usage::usage);
        let installed = s.spawn(apps::scan);
        let users = s.spawn(users::users);
        let activity = s.spawn(activity::activity);

        let identity = identity.join().unwrap_or_default();
        let timeline = history.join().unwrap_or_default();
        let power = power.join().unwrap_or_default();
        let ssd = ssd.join().unwrap_or_default();
        let disk = disk.join().unwrap_or_default();
        let memory = memory.join().unwrap_or_default();
        let battery = battery.join().unwrap_or_default();
        let network = network.join().unwrap_or_default();
        let usage = usage.join().unwrap_or_default();
        let installed = installed.join().unwrap_or_default();
        let users = users.join().unwrap_or_default();
        let activity = activity.join().unwrap_or_default();

        let mut usage = usage;
        apps::name_usage(&mut usage, &installed);

        let mut household = Household::default();
        apps::fill(&mut household, &installed, &usage);
        users::fill(&mut household, users);
        activity::fill(&mut household, &activity);

        let mut memory = memory;
        if let Some(memory) = memory.as_mut() {
            memory.top = activity.top_memory.clone();
        }

        Report {
            generated_at: Local::now(),
            identity,
            timeline,
            power,
            ssd,
            disk,
            memory,
            battery,
            network,
            usage,
            household,
        }
    })
}
