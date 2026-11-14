use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use std::time::Duration;
use std::time::Instant;

use crate::sgdb::Page;

const TTL: Duration = Duration::from_secs(600);
const MAX_PAGES: usize = 128;

static PAGES: LazyLock<Mutex<HashMap<String, (Instant, Page)>>> = LazyLock::new(Mutex::default);

fn pages() -> MutexGuard<'static, HashMap<String, (Instant, Page)>> {
    PAGES.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn lookup(key: &str) -> Option<Page> {
    pages()
        .get(key)
        .filter(|(at, _)| at.elapsed() < TTL)
        .map(|(_, page)| page.clone())
}

pub fn store(key: String, page: &Page) {
    let mut pages = pages();
    if pages.len() >= MAX_PAGES {
        pages.retain(|_, (at, _)| at.elapsed() < TTL);
    }
    if pages.len() >= MAX_PAGES {
        pages.clear();
    }

    pages.insert(key, (Instant::now(), page.clone()));
}

pub fn clear() {
    pages().clear();
}

#[cfg(test)]
#[path = "../tests/cache.rs"]
mod tests;
