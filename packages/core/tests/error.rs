use super::*;

#[test]
fn http_statuses_become_typed_errors() {
    assert!(matches!(Error::from(ureq::Error::StatusCode(401)), Error::KeyRejected));
    assert!(matches!(Error::from(ureq::Error::StatusCode(403)), Error::KeyRejected));
    assert!(matches!(Error::from(ureq::Error::StatusCode(429)), Error::RateLimited));
    assert!(matches!(Error::from(ureq::Error::StatusCode(500)), Error::Status(500)));
}

#[test]
fn codes_are_stable_and_messages_are_for_people() {
    assert_eq!(Error::NoApiKey.code(), "no_api_key");
    assert_eq!(Error::KeyRejected.code(), "key_rejected");
    assert_eq!(Error::Status(500).code(), "api");
    assert_eq!(Error::Status(500).to_string(), "SteamGridDB returned HTTP 500");

    let io = Error::io("saving the settings")(io::Error::other("disk full"));
    assert_eq!(io.code(), "io");
    assert_eq!(io.to_string(), "saving the settings: disk full");
}
