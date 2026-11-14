use super::*;

// one test: the cache is shared by the whole process
#[test]
fn pages_are_served_until_forgotten_and_the_cache_stays_bounded() {
    let page = Page {
        items: Vec::new(),
        more: true,
    };

    assert!(lookup("a").is_none());
    store("a".to_string(), &page);
    assert_eq!(lookup("a"), Some(page.clone()));
    assert!(lookup("b").is_none());

    clear();
    assert!(lookup("a").is_none());

    for n in 0..=MAX_PAGES {
        store(format!("page {n}"), &page);
    }
    assert!(pages().len() <= MAX_PAGES);
    assert_eq!(
        lookup(&format!("page {MAX_PAGES}")),
        Some(page),
        "the newest page is kept"
    );

    clear();
}
