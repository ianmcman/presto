//! Verify that engine/main.js has the correct MediaSession flags.

#[test]
fn engine_media_session_disabled() {
    // Read the engine main.js file relative to CARGO_MANIFEST_DIR
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let engine_main = std::path::Path::new(&manifest_dir)
        .parent()
        .expect("no parent of manifest dir")
        .parent()
        .expect("no grandparent of manifest dir")
        .join("engine")
        .join("main.js");

    let contents = std::fs::read_to_string(&engine_main)
        .expect("could not read engine/main.js");

    // Verify the file contains both the feature flags and the condition
    assert!(
        contents.contains("MediaSessionService,HardwareMediaKeyHandling"),
        "engine/main.js must contain 'MediaSessionService,HardwareMediaKeyHandling' in disable-features"
    );

    assert!(
        contents.contains("!args.keepMediaSession"),
        "engine/main.js must contain '!args.keepMediaSession' to gate the disable-features"
    );

    // Verify they're on the same disable-features line
    let lines: Vec<&str> = contents.lines().collect();
    let mut found_condition = false;
    let mut found_features = false;

    for (i, line) in lines.iter().enumerate() {
        if line.contains("disable-features") {
            // Check if this line has both the condition and the features
            if line.contains("MediaSessionService,HardwareMediaKeyHandling")
                && line.contains("!args.keepMediaSession")
            {
                // Both are on the same line
                found_condition = true;
                found_features = true;
                break;
            } else if line.contains("MediaSessionService,HardwareMediaKeyHandling") {
                found_features = true;
            }

            // Check a few nearby lines for the condition
            for nearby_line in lines.iter().skip(i.saturating_sub(2)).take(5) {
                if nearby_line.contains("!args.keepMediaSession") {
                    found_condition = true;
                }
            }
        }
    }

    assert!(
        found_features && found_condition,
        "engine/main.js must have MediaSessionService,HardwareMediaKeyHandling and !args.keepMediaSession \
        on or near the same disable-features line"
    );
}
