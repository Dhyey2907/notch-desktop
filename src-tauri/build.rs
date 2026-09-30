fn main() {
    embed_oauth_clients();
    tauri_build::build()
}

/// "Continue with Google / Spotify / GitHub" needs Notch's own registered OAuth client IDs.
/// They live in `oauth-clients.json` next to this file (gitignored; see
/// `oauth-clients.example.json`) and are baked into the build as environment variables. Without
/// the file, Settings falls back to "paste your own client ID".
fn embed_oauth_clients() {
    println!("cargo:rerun-if-changed=oauth-clients.json");
    let Ok(text) = std::fs::read_to_string("oauth-clients.json") else {
        return;
    };
    let json: serde_json::Value = match serde_json::from_str(&text) {
        Ok(json) => json,
        Err(error) => {
            println!("cargo:warning=oauth-clients.json isn't valid JSON ({error}); one-click sign-in is off");
            return;
        }
    };
    let pairs = [
        ("/google/clientId", "NOTCH_GOOGLE_CLIENT_ID"),
        ("/google/clientSecret", "NOTCH_GOOGLE_CLIENT_SECRET"),
        ("/spotify/clientId", "NOTCH_SPOTIFY_CLIENT_ID"),
        ("/github/clientId", "NOTCH_GITHUB_CLIENT_ID"),
    ];
    for (pointer, var) in pairs {
        let value = json
            .pointer(pointer)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("");
        // Placeholders from the example file don't count.
        if !value.is_empty() && !value.starts_with('<') {
            println!("cargo:rustc-env={var}={value}");
        }
    }
}
