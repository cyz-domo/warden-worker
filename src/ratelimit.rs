use std::cell::RefCell;
use std::collections::HashMap;

/// Prelogin answers with the stored account's real KDF type and iterations, so an
/// unauthenticated caller can use it to enumerate accounts and pick the cheapest
/// offline attack shape. Count requests per client address instead of answering
/// them at full cost.
const WINDOW_MILLIS: u64 = 60_000;
const MAX_REQUESTS_PER_WINDOW: u32 = 30;
const MAX_TRACKED_CLIENTS: usize = 4096;

thread_local! {
    // Counters live in the isolate that serves the request, so they cost no D1
    // writes and no cross-isolate coordination. A recycled isolate starts a fresh
    // window, which is acceptable for a best effort limit.
    static WINDOWS: RefCell<HashMap<String, (u64, u32)>> = RefCell::new(HashMap::new());
}

pub fn prelogin_allowed(client: Option<&str>) -> bool {
    // Without a client address every caller would share one bucket, so fail open.
    let Some(client) = client else {
        return true;
    };

    let window = js_sys::Date::now() as u64 / WINDOW_MILLIS;
    WINDOWS.with(|windows| {
        let mut windows = windows.borrow_mut();
        if windows.len() > MAX_TRACKED_CLIENTS {
            windows.retain(|_, seen_window| seen_window.0 == window);
            if windows.len() > MAX_TRACKED_CLIENTS {
                windows.clear();
            }
        }

        let entry = windows.entry(client.to_string()).or_insert((window, 0));
        if entry.0 != window {
            entry.0 = window;
            entry.1 = 1;
            return true;
        }
        if entry.1 >= MAX_REQUESTS_PER_WINDOW {
            return false;
        }
        entry.1 += 1;
        true
    })
}
