use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// ---- tone3000 publishable key -----------------------------------------------
// Safe to embed: this key identifies the app to tone3000 but is not secret.
// Obtain from https://www.tone3000.com developer dashboard.
pub const TONE3000_CLIENT_ID: &str = "t3k_pub_5Wf8b5OvqtEZMxMHfKrjt2oriDf7-2zW";
pub const TONE3000_BASE_URL: &str = "https://www.tone3000.com/api/v1";

// ---- API response types -----------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tone3000EmbeddedUser {
    pub username: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tone3000Tone {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub gear: String,
    pub user: Tone3000EmbeddedUser,
    pub downloads_count: i64,
    pub favorites_count: i64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tone3000Model {
    pub id: i64,
    pub tone_id: i64,
    pub name: String,
    pub size: Option<String>,
    pub architecture_version: String,
    pub model_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub tones: Vec<Tone3000Tone>,
    pub total: i64,
    pub page: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
}

// ---- Internal persistence types ---------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub username: String,
    pub avatar_url: Option<String>,
}

// ---- In-flight OAuth state --------------------------------------------------

pub struct PendingAuth {
    pub verifier: String,
    pub state_token: String,
}

pub struct DiscoverState(pub Mutex<Option<PendingAuth>>);

// ---- Pagination wrapper (internal) ------------------------------------------

#[derive(Deserialize)]
struct Page<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i32,
}

// ---- PKCE helpers -----------------------------------------------------------

pub fn generate_pkce() -> (String, String) {
    use rand::Rng;
    use sha2::{Digest, Sha256};
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    let verifier: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(43)
        .map(char::from)
        .collect();

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(&hash);

    (verifier, challenge)
}

pub fn generate_state_token() -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

// ---- Token file I/O ---------------------------------------------------------

pub fn load_tokens(tokens_path: &std::path::Path) -> Option<StoredTokens> {
    let data = std::fs::read_to_string(tokens_path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_tokens(tokens_path: &std::path::Path, tokens: &StoredTokens) {
    if let Ok(json) = serde_json::to_string_pretty(tokens) {
        let _ = std::fs::write(tokens_path, json);
    }
}

pub fn delete_tokens(tokens_path: &std::path::Path) {
    let _ = std::fs::remove_file(tokens_path);
}

// ---- Auth Tauri commands ----------------------------------------------------

use tauri::{AppHandle, Manager, State};
use crate::store::AppDirs;

fn app_dirs(app: &AppHandle) -> Result<AppDirs, String> {
    let path = app.path().app_data_dir()
        .map_err(|e| format!("No app data dir: {e}"))?;
    AppDirs::new(&path).map_err(|e| format!("Failed to create dirs: {e}"))
}

/// Step 1 of OAuth: generate PKCE, open browser, return (nothing — frontend waits for event).
#[tauri::command]
pub fn tone3000_auth_start(
    app: AppHandle,
    state: State<'_, DiscoverState>,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let (verifier, challenge) = generate_pkce();
    let state_token = generate_state_token();

    *state.0.lock().unwrap() = Some(PendingAuth {
        verifier: verifier.clone(),
        state_token: state_token.clone(),
    });

    let auth_url = format!(
        "{}/oauth/authorize?client_id={}&redirect_uri=daisy://oauth-callback\
         &response_type=code&code_challenge={}&code_challenge_method=S256\
         &state={}&platform=nam",
        TONE3000_BASE_URL, TONE3000_CLIENT_ID, challenge, state_token
    );

    app.opener()
        .open_url(&auth_url, None::<&str>)
        .map_err(|e| format!("Cannot open browser: {e}"))?;

    Ok(())
}

/// Cancel a pending OAuth flow.
#[tauri::command]
pub fn tone3000_auth_cancel(state: State<'_, DiscoverState>) {
    *state.0.lock().unwrap() = None;
}

/// Return current auth status (username if logged in, None if not).
#[tauri::command]
pub fn tone3000_check_auth(app: AppHandle) -> Result<AuthStatus, String> {
    let dirs = app_dirs(&app)?;
    match load_tokens(&dirs.tokens_json()) {
        Some(t) => Ok(AuthStatus {
            authenticated: true,
            username: Some(t.username),
            avatar_url: t.avatar_url,
        }),
        None => Ok(AuthStatus { authenticated: false, username: None, avatar_url: None }),
    }
}

/// Clear stored tokens.
#[tauri::command]
pub fn tone3000_sign_out(app: AppHandle) -> Result<(), String> {
    let dirs = app_dirs(&app)?;
    delete_tokens(&dirs.tokens_json());
    Ok(())
}

// ---- OAuth callback + token refresh -----------------------------------------

/// Internal: called from the deep-link handler with the full callback URL.
/// Exchanges the auth code for tokens, persists them, emits tone3000-auth-result.
pub async fn handle_oauth_callback(app: &AppHandle, url: &str) -> Result<StoredTokens, String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid callback URL: {e}"))?;
    let params: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();

    if let Some(error) = params.get("error") {
        return Err(format!("OAuth error: {error}"));
    }

    let code  = params.get("code").ok_or("Missing code in callback")?;
    let state = params.get("state").ok_or("Missing state in callback")?;

    let discover_state = app.state::<DiscoverState>();
    let pending = discover_state.0.lock().unwrap().take()
        .ok_or("No pending auth — unexpected callback")?;

    if pending.state_token != *state {
        return Err("OAuth state mismatch — possible CSRF".into());
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{TONE3000_BASE_URL}/oauth/token"))
        .form(&[
            ("grant_type",    "authorization_code"),
            ("code",          code.as_str()),
            ("code_verifier", pending.verifier.as_str()),
            ("redirect_uri",  "daisy://oauth-callback"),
            ("client_id",     TONE3000_CLIENT_ID),
        ])
        .send()
        .await
        .map_err(|e| format!("Token exchange request failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed: {body}"));
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: String,
        expires_in: i64,
    }
    let token_resp: TokenResponse = resp.json().await
        .map_err(|e| format!("Failed to parse token response: {e}"))?;

    let user_resp = client
        .get(format!("{TONE3000_BASE_URL}/user"))
        .bearer_auth(&token_resp.access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch user: {e}"))?;

    #[derive(Deserialize)]
    struct UserResponse {
        username: String,
        avatar_url: Option<String>,
    }
    let user: UserResponse = user_resp.json().await
        .map_err(|e| format!("Failed to parse user response: {e}"))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let tokens = StoredTokens {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_at: now + token_resp.expires_in,
        username: user.username,
        avatar_url: user.avatar_url,
    };

    let dirs = app_dirs(app)?;
    save_tokens(&dirs.tokens_json(), &tokens);

    Ok(tokens)
}

/// Internal: get a valid access token, refreshing if needed.
pub async fn get_access_token(app: &AppHandle) -> Result<String, String> {
    let dirs = app_dirs(app)?;
    let mut tokens = load_tokens(&dirs.tokens_json())
        .ok_or("Not authenticated")?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    if tokens.expires_at - now < 60 {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{TONE3000_BASE_URL}/oauth/token"))
            .form(&[
                ("grant_type",    "refresh_token"),
                ("refresh_token", tokens.refresh_token.as_str()),
                ("client_id",     TONE3000_CLIENT_ID),
            ])
            .send()
            .await
            .map_err(|e| format!("Refresh request failed: {e}"))?;

        if resp.status() == reqwest::StatusCode::BAD_REQUEST {
            delete_tokens(&dirs.tokens_json());
            return Err("SESSION_EXPIRED".into());
        }

        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            refresh_token: String,
            expires_in: i64,
        }
        let refreshed: RefreshResponse = resp.json().await
            .map_err(|e| format!("Failed to parse refresh response: {e}"))?;

        tokens.access_token = refreshed.access_token;
        tokens.refresh_token = refreshed.refresh_token;
        tokens.expires_at = now + refreshed.expires_in;
        save_tokens(&dirs.tokens_json(), &tokens);
    }

    Ok(tokens.access_token)
}

// ---- Search and list_models commands ----------------------------------------

/// Search tone3000 for Daisy-compatible tones.
/// Always filters: platform=nam, architecture=2.
#[tauri::command]
pub async fn tone3000_search(
    app: AppHandle,
    query: Option<String>,
    gear: Option<String>,
    sort: Option<String>,
    page: Option<u32>,
) -> Result<SearchResult, String> {
    let token = get_access_token(&app).await?;

    let sort_val = sort.as_deref().unwrap_or("trending");
    let page_val = page.unwrap_or(1);

    let mut url = format!(
        "{TONE3000_BASE_URL}/tones/search?platform=nam&architecture=2\
         &sort={sort_val}&page={page_val}&page_size=25"
    );
    if let Some(q) = &query {
        url.push_str(&format!("&query={}", urlencoding::encode(q)));
    }
    if let Some(g) = &gear {
        url.push_str(&format!("&gears={g}"));
    }

    let client = reqwest::Client::new();
    let resp = client.get(&url).bearer_auth(&token).send().await
        .map_err(|e| format!("Search request failed: {e}"))?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        let dirs = app_dirs(&app)?;
        delete_tokens(&dirs.tokens_json());
        return Err("SESSION_EXPIRED".into());
    }
    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("RATE_LIMITED".into());
    }
    if !resp.status().is_success() {
        return Err(format!("Search failed: HTTP {}", resp.status()));
    }

    let body = resp.text().await
        .map_err(|e| format!("Failed to read search response: {e}"))?;
    let page_resp: Page<Tone3000Tone> = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse search response: {e}\nBody: {body}"))?;

    Ok(SearchResult {
        tones: page_resp.data,
        total: page_resp.total,
        page: page_resp.page,
    })
}

/// List models for a tone, filtered to nano + NAM A2.
#[tauri::command]
pub async fn tone3000_list_models(
    app: AppHandle,
    tone_id: i64,
) -> Result<Vec<Tone3000Model>, String> {
    let token = get_access_token(&app).await?;

    let url = format!(
        "{TONE3000_BASE_URL}/models?tone_id={tone_id}&page_size=100"
    );
    let client = reqwest::Client::new();
    let resp = client.get(&url).bearer_auth(&token).send().await
        .map_err(|e| format!("Models request failed: {e}"))?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        let dirs = app_dirs(&app)?;
        delete_tokens(&dirs.tokens_json());
        return Err("SESSION_EXPIRED".into());
    }
    if !resp.status().is_success() {
        return Err(format!("Models request failed: HTTP {}", resp.status()));
    }

    let body = resp.text().await
        .map_err(|e| format!("Failed to read models response: {e}"))?;
let page_resp: Page<Tone3000Model> = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse models response: {e}\nBody: {body}"))?;

    Ok(page_resp.data)
}

// ---- download_tone command --------------------------------------------------

#[tauri::command]
pub async fn download_tone(
    app: AppHandle,
    model_id: i64,
    tone_id: i64,
) -> Result<crate::types::ModelInfo, String> {
    use chrono::Utc;
    use tauri_plugin_shell::ShellExt;

    let token = get_access_token(&app).await?;
    let dirs = app_dirs(&app)?;

    let client = reqwest::Client::new();
    let model_resp = client
        .get(format!("{TONE3000_BASE_URL}/models/{model_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("Model fetch failed: {e}"))?;

    if model_resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        delete_tokens(&dirs.tokens_json());
        return Err("SESSION_EXPIRED".into());
    }
    if !model_resp.status().is_success() {
        return Err(format!("Model fetch failed: HTTP {}", model_resp.status()));
    }

    let model: Tone3000Model = model_resp.json().await
        .map_err(|e| format!("Failed to parse model: {e}"))?;

    let nam_bytes = client
        .get(&model.model_url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download: {e}"))?;

    let run_id   = uuid::Uuid::new_v4().to_string();
    let tmp_nam  = dirs.tmp.join(format!("{run_id}_input.nam"));
    let tmp_namb = dirs.tmp.join(format!("{run_id}_output.namb"));

    std::fs::write(&tmp_nam, &nam_bytes)
        .map_err(|e| format!("Failed to write temp file: {e}"))?;

    let output = app.shell()
        .sidecar("nam2namb")
        .map_err(|e| format!("nam2namb not found: {e}"))?
        .args([
            "--slim", "0.5",
            tmp_nam.to_str().unwrap(),
            tmp_namb.to_str().unwrap(),
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run nam2namb: {e}"))?;

    let _ = std::fs::remove_file(&tmp_nam);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(format!("Conversion failed: {stderr}"));
    }

    let name = model.name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    let name = if name.is_empty() { format!("tone_{tone_id}") } else { name };

    let id   = uuid::Uuid::new_v4().to_string();
    let dest = dirs.model_path(&id, &name);
    std::fs::rename(&tmp_namb, &dest)
        .or_else(|_| std::fs::copy(&tmp_namb, &dest).map(|_| ())
            .and_then(|_| std::fs::remove_file(&tmp_namb)))
        .map_err(|e| format!("Failed to move file: {e}"))?;

    let size_bytes = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let info = crate::types::ModelInfo {
        id,
        name,
        size_bytes,
        imported_at: Utc::now().to_rfc3339(),
        tone3000_id: Some(tone_id.to_string()),
        tone3000_model_id: Some(model_id.to_string()),
    };

    let mut models = crate::store::load_models(&dirs);
    models.push(info.clone());
    crate::store::save_models(&dirs, &models);
    Ok(info)
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};

    #[test]
    fn pkce_verifier_is_43_chars() {
        let (verifier, _) = generate_pkce();
        assert_eq!(verifier.len(), 43);
    }

    #[test]
    fn pkce_challenge_is_valid_base64url() {
        let (_, challenge) = generate_pkce();
        let decoded = URL_SAFE_NO_PAD.decode(&challenge).unwrap();
        assert_eq!(decoded.len(), 32);
    }

    #[test]
    fn pkce_challenge_is_sha256_of_verifier() {
        let (verifier, challenge) = generate_pkce();
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(&hasher.finalize());
        assert_eq!(challenge, expected);
    }

    #[test]
    fn state_token_is_32_chars() {
        let state = generate_state_token();
        assert_eq!(state.len(), 32);
    }

    #[test]
    fn token_roundtrip() {
        let tmp = std::env::temp_dir().join("test_tokens.json");
        let tokens = StoredTokens {
            access_token: "acc".into(),
            refresh_token: "ref".into(),
            expires_at: 9999999999,
            username: "testuser".into(),
            avatar_url: None,
        };
        save_tokens(&tmp, &tokens);
        let loaded = load_tokens(&tmp).unwrap();
        assert_eq!(loaded.access_token, "acc");
        assert_eq!(loaded.username, "testuser");
        let _ = std::fs::remove_file(&tmp);
    }
}
