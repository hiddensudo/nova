use super::*;

#[test]
fn merged_application_keeps_icon_metadata_from_any_of_its_streams() {
    let inputs: Vec<Stream> = serde_json::from_value(serde_json::json!([
        {"index": 1, "sink": 2, "corked": true, "properties": {
            "application.id": "com.spotify.Client", "application.name": "Spotify"
        }},
        {"index": 2, "sink": 2, "properties": {
            "application.id": "com.spotify.Client", "application.name": "Spotify",
            "application.icon_name": "spotify-client", "application.process.binary": "spotify"
        }}
    ]))
    .unwrap();
    let apps = streams(&inputs, &Config::default());
    assert_eq!(apps.len(), 1);
    assert!(apps[0].playing);
    assert_eq!(apps[0].key, "com.spotify.Client");
    assert_eq!(apps[0].icon_name.as_deref(), Some("spotify-client"));
    assert_eq!(apps[0].binary.as_deref(), Some("spotify"));
}
