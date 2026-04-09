use crate::base::{device_name, MCRSError};
use crate::cli::Args;
use crate::settings::{ProfileConfig, Settings};
use crate::{
    build_matrix_client, get_homeserver, get_password, get_room_default, get_user_login, Error,
    LoginCLI,
};
use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::ruma::api::client::error::ErrorKind;
use matrix_sdk::ruma::OwnedUserId;
use matrix_sdk::Client;
use std::fmt;
use tracing::{debug, error};

#[derive(Debug)]
pub enum LoginType {
    Password(String),
    Sso,
}

impl fmt::Display for LoginType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct Login {
    pub client: Client,
    pub settings: Settings,
}

impl Login {
    pub fn new(client: Client, settings: Settings) -> Self {
        Login { client, settings }
    }

    pub async fn login_and_sync(&mut self, login_type: LoginType) -> Result<(), MCRSError> {
        let client = self.client.clone();
        match login_type {
            LoginType::Password(pass) => {
                let user_id = self.settings.profile.user_id.clone();
                let resp = client
                    .matrix_auth()
                    .login_username(&user_id, &pass)
                    .initial_device_display_name(device_name().as_str())
                    .send()
                    .await
                    .map_err(|e| error!("{}", e));
                let session = MatrixSession::from(&resp.unwrap());
                self.settings.write_session(session)?;
            }
            LoginType::Sso => {
                let mut login = client
                    .matrix_auth()
                    .login_sso(|url| {
                        let opened = format!(
                            "The following URL should have been opened in your browser:\n {url}"
                        );
                        async move {
                            tokio::task::spawn_blocking(move || open::that(url));
                            println!("\n{opened}\n");
                            Ok(())
                        }
                    })
                    .initial_device_display_name(device_name().as_str());
                if let Ok(prev_session) = self
                    .settings
                    .read_session(self.settings.session_json.as_path())
                {
                    login = login.device_id(prev_session.device_id.as_ref());
                    println!("{}", prev_session.device_id);
                }
                let resp = login.send().await.map_err(MCRSError::from);
                match &resp {
                    Err(MCRSError::Matrix(new_e)) => {
                        if let Some(ErrorKind::UnknownToken { .. }) = new_e.client_api_error_kind()
                        {
                            println!("{}", self.settings.session_json.display());
                        }
                    }
                    Ok(_resp) => (),
                    Err(e) => println!("{}", e),
                };
                let resp = resp?;
                let session = MatrixSession::from(&resp);
                self.settings.write_session(session)?;
            }
        }
        Ok(())
    }
}

pub async fn cli_login(
    client: Client,
    settings: Settings,
    login_type: LoginType,
) -> Result<Client, MCRSError> {
    let mut login = Login::new(client, settings);
    login.login_and_sync(login_type).await?;
    Ok(login.client)
}

/// Try to restore an existing session from the session file on disk.
///
/// Returns:
/// - `Ok(Some(client))` — session found and restored successfully.
/// - `Ok(None)`         — no session file exists yet (first-time login required).
/// - `Err(_)`           — session file exists but could not be read/restored.
pub async fn try_restore_session(
    client: Client,
    settings: &Settings,
) -> Result<Option<Client>, MCRSError> {
    if !settings.session_json.exists() {
        return Ok(None);
    }
    let session = settings.read_session(&settings.session_json)?;
    let matrix_session: MatrixSession = session.into();
    client.restore_session(matrix_session).await?;
    Ok(Some(client))
}

pub async fn check_session(client: &Client) -> bool {
    client.whoami().await.is_ok()
}

pub struct LoginFlow;
impl LoginFlow {
    async fn already_logged_in(settings: &Settings, client: &Client) -> bool {
        if let Ok(Some(restored)) = try_restore_session(client.clone(), settings).await {
            if check_session(&restored).await {
                return true;
            }
        }
        false
    }
    async fn login_password(ap: &mut Args) -> Result<(Settings, Client), Error> {
        let profile_name = ap.profile.clone().unwrap_or_else(|| "default".to_string());

        if let Some(existing) = Settings::try_load_profile(&profile_name) {
            println!("Found existing profile '{profile_name}' in config:");
            if ap.homeserver.is_none() {
                if let Some(ref hs) = existing.profile.homeserver {
                    println!("  homeserver   : {hs}  (override with --homeserver)");
                    ap.homeserver = Some(hs.clone());
                }
            }
            if ap.user_login.is_none() {
                let uid = existing.profile.user_id.to_string();
                println!("  user-login   : {uid}  (override with --user-login)");
                ap.user_login = Some(uid);
            }
            if ap.room_default.is_none() {
                let room = existing.profile.default_room.clone();
                println!("  room-default : {room}  (override with --room-default)");
                ap.room_default = Some(room);
            }
            println!();
        }

        get_homeserver(ap);
        get_user_login(ap);
        get_password(ap);
        get_room_default(ap);

        let user_id: OwnedUserId =
            ap.user_login
                .as_deref()
                .unwrap_or("")
                .parse()
                .map_err(|_| {
                    error!("Invalid Matrix user ID format");
                    Error::MissingUser
                })?;

        let profile = ProfileConfig {
            user_id,
            homeserver: ap.homeserver.clone(),
            default_room: ap.room_default.clone().unwrap_or_default(),
            dirs: None,
        };
        let settings = Settings::create_or_update_profile(&profile_name, profile).map_err(|e| {
            error!("Failed to save profile to config: {e}");
            Error::NoCredentialsFound
        })?;

        let homeserver = settings
            .profile
            .homeserver
            .clone()
            .ok_or(Error::HomeserverNotSet)?;
        let client = build_matrix_client(&homeserver, &settings, ap.timeout).await?;

        if Self::already_logged_in(&settings, &client).await {
            println!(
                "Already logged in as {}. \
                     Use --logout me to end the current session before logging in again.",
                settings.profile.user_id
            );
            return Err(Error::LoginFailed);
        }

        let pass = ap.password.clone().unwrap_or_default();
        let client = cli_login(client, settings.clone(), LoginType::Password(pass))
            .await
            .map_err(|e| {
                error!("Login failed: {e}");
                Error::LoginFailed
            })?;

        Ok((settings, client))
    }

    async fn login_sso(ap: &mut Args) -> Result<(Settings, Client), Error> {
        let settings = Settings::load(ap).map_err(|e| {
            error!("Failed to load settings: {e}");
            Error::NoCredentialsFound
        })?;
        let homeserver = settings
            .profile
            .homeserver
            .clone()
            .ok_or(Error::HomeserverNotSet)?;
        let client = build_matrix_client(&homeserver, &settings, ap.timeout).await?;

        if Self::already_logged_in(&settings, &client).await {
            println!(
                "Already logged in as {}. \
                     Use --logout me to end the current session before logging in again.",
                settings.profile.user_id
            );
            return Err(Error::LoginFailed);
        }
        use crate::base::MCRSError;
        use matrix_sdk::ruma::api::client::error::ErrorKind;
        let client = cli_login(client, settings.clone(), LoginType::Sso)
            .await
            .map_err(|e| {
                if let MCRSError::Matrix(new_e) = e {
                    if let Some(ErrorKind::UnknownToken { .. }) = new_e.client_api_error_kind() {
                        println!("{}", settings.session_json.display());
                    }
                }
                Error::LoginFailed
            })?;

        Ok((settings, client))
    }

    async fn login_restore(ap: &mut Args) -> Result<(Settings, Client), Error> {
        let settings = Settings::load(ap).map_err(|e| {
            error!("Failed to load settings: {e}");
            Error::NoCredentialsFound
        })?;
        let homeserver = settings
            .profile
            .homeserver
            .clone()
            .ok_or(Error::HomeserverNotSet)?;
        let client = build_matrix_client(&homeserver, &settings, ap.timeout).await?;
        let client = match try_restore_session(client, &settings).await {
            Ok(Some(c)) => {
                debug!("Session restored from {:?}", settings.session_json);
                c
            }
            Ok(None) => {
                error!("No session file found. Please run with --login first.");
                return Err(Error::NotLoggedIn);
            }
            Err(e) => {
                error!("Failed to restore session: {e}");
                return Err(Error::RestoreLoginFailed);
            }
        };
        Ok((settings, client))
    }
    pub async fn login(ap: &mut Args) -> Result<(Settings, Client), Error> {
        let login_cli = ap.login;

        let (settings, client) = match login_cli {
            LoginCLI::None => Self::login_restore(ap).await?,
            LoginCLI::Password => Self::login_password(ap).await?,
            LoginCLI::Sso => Self::login_sso(ap).await?,
        };
        Ok((settings, client))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{DirectoryValues, ProfileConfig, Settings};
    use matrix_sdk::Client;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    // ─── helpers ──────────────────────────────────────────────────────────────

    /// Start a `wiremock` `MockServer` that handles the two HTTP round-trips
    /// that `matrix_sdk::Client::builder().build()` performs:
    ///
    /// * `GET /.well-known/matrix/client` → 404 so matrix-sdk uses the URL as-is
    /// * `GET /_matrix/client/versions`   → minimal valid JSON
    ///
    /// Any other request receives a default 404 response from wiremock and will
    /// not cause the test to panic.
    async fn start_mock_homeserver() -> MockServer {
        let server = MockServer::start().await;

        // Disable OIDC / client-discovery; matrix-sdk will use the raw URL.
        Mock::given(method("GET"))
            .and(path("/.well-known/matrix/client"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        // Satisfy the server-capabilities check.
        Mock::given(method("GET"))
            .and(path("/_matrix/client/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "versions": ["v1.0", "v1.1", "v1.2", "v1.3", "v1.5"],
                "unstable_features": {}
            })))
            .mount(&server)
            .await;

        server
    }

    /// Build a `matrix_sdk::Client` pointed at `homeserver_url` using a
    /// temporary SQLite store directory.
    async fn build_test_client(homeserver_url: &str, sqlite_dir: &std::path::Path) -> Client {
        Client::builder()
            .homeserver_url(homeserver_url)
            .sqlite_store(sqlite_dir, None)
            .build()
            .await
            .expect("failed to build test matrix_sdk::Client")
    }

    /// Minimal `Settings` whose `session_json` lives at `session_path`.
    fn make_test_settings(homeserver_url: &str, session_path: PathBuf) -> Settings {
        Settings {
            session_json: session_path,
            sqlite_dir: PathBuf::from("/tmp/mcrs_test_sqlite"),
            profile_name: "test".to_string(),
            profile: ProfileConfig {
                user_id: "@testuser:localhost".parse().unwrap(),
                homeserver: Some(homeserver_url.parse().unwrap()),
                default_room: "!room:localhost".to_string(),
                dirs: None,
            },
            dirs: DirectoryValues {
                cache: PathBuf::from("/tmp/mcrs_test_cache"),
                data: PathBuf::from("/tmp/mcrs_test_data"),
                logs: PathBuf::from("/tmp/mcrs_test_logs"),
            },
        }
    }

    // ─── LoginType display / debug tests ──────────────────────────────────────

    /// `LoginType::Password` renders with "Password" in its `Display` output.
    #[test]
    fn test_login_type_password_display_contains_password() {
        let lt = LoginType::Password("s3cr3t".to_string());
        let s = lt.to_string();
        assert!(
            s.contains("Password"),
            "expected 'Password' in Display string, got '{s}'"
        );
    }

    /// `LoginType::Sso` renders as exactly "Sso" via `Display`.
    #[test]
    fn test_login_type_sso_display_is_sso() {
        assert_eq!(LoginType::Sso.to_string(), "Sso");
    }

    /// `LoginType::Password` mentions "Password" in its `Debug` output.
    #[test]
    fn test_login_type_password_debug_contains_password() {
        let dbg = format!("{:?}", LoginType::Password("top_secret".to_string()));
        assert!(
            dbg.contains("Password"),
            "Debug should mention 'Password', got '{dbg}'"
        );
    }

    /// `LoginType::Sso` mentions "Sso" in its `Debug` output.
    #[test]
    fn test_login_type_sso_debug_contains_sso() {
        let dbg = format!("{:?}", LoginType::Sso);
        assert!(
            dbg.contains("Sso"),
            "Debug should mention 'Sso', got '{dbg}'"
        );
    }

    /// Two different `LoginType::Password` values stringify differently so the
    /// Display impl is not constant.
    #[test]
    fn test_login_type_password_display_reflects_the_variant() {
        let sso = LoginType::Sso.to_string();
        let pwd = LoginType::Password("x".to_string()).to_string();
        assert_ne!(
            sso, pwd,
            "Password and Sso must produce different Display strings"
        );
    }

    // ─── Login::new constructor test ──────────────────────────────────────────

    /// `Login::new` stores the settings it receives without modification.
    #[tokio::test]
    async fn test_login_new_stores_settings_unchanged() {
        let mock_server = start_mock_homeserver().await;
        let tmpdir = TempDir::new().unwrap();

        let client = build_test_client(&mock_server.uri(), tmpdir.path()).await;
        let settings = make_test_settings(&mock_server.uri(), tmpdir.path().join("session.json"));

        let profile_name = settings.profile_name.clone();
        let login = Login::new(client, settings);

        assert_eq!(
            login.settings.profile_name, profile_name,
            "Login::new must store the provided settings unchanged"
        );
    }

    // ─── try_restore_session tests ────────────────────────────────────────────

    /// When `session_json` does not exist `try_restore_session` must return
    /// `Ok(None)` without performing any server communication.
    #[tokio::test]
    async fn test_try_restore_session_returns_none_when_no_session_file() {
        let mock_server = start_mock_homeserver().await;
        let tmpdir = TempDir::new().unwrap();

        let client = build_test_client(&mock_server.uri(), tmpdir.path()).await;

        // Point to a path that does not exist on disk.
        let missing = tmpdir.path().join("no_such_session.json");
        assert!(
            !missing.exists(),
            "pre-condition: session file must be absent"
        );

        let settings = make_test_settings(&mock_server.uri(), missing);
        let result = try_restore_session(client, &settings).await;

        assert!(
            result.is_ok(),
            "expected Ok(..), got Err: {:?}",
            result.err()
        );
        assert!(
            result.unwrap().is_none(),
            "expected Ok(None) when the session file is absent"
        );
    }

    /// When `session_json` exists but its contents are not valid JSON,
    /// `try_restore_session` must return an `Err`.
    #[tokio::test]
    async fn test_try_restore_session_returns_error_for_corrupt_session_file() {
        let mock_server = start_mock_homeserver().await;
        let tmpdir = TempDir::new().unwrap();

        let client = build_test_client(&mock_server.uri(), tmpdir.path()).await;

        // Create a session file filled with garbage.
        let session_file = tmpdir.path().join("corrupt_session.json");
        std::fs::write(&session_file, b"{ not: valid json }").unwrap();
        assert!(
            session_file.exists(),
            "pre-condition: corrupt file must exist"
        );

        let settings = make_test_settings(&mock_server.uri(), session_file);
        let result = try_restore_session(client, &settings).await;

        assert!(
            result.is_err(),
            "expected Err for corrupt session JSON, got Ok"
        );
    }

    /// When `session_json` exists but is completely empty,
    /// `try_restore_session` must return an `Err`.
    #[tokio::test]
    async fn test_try_restore_session_returns_error_for_empty_session_file() {
        let mock_server = start_mock_homeserver().await;
        let tmpdir = TempDir::new().unwrap();

        let client = build_test_client(&mock_server.uri(), tmpdir.path()).await;

        let session_file = tmpdir.path().join("empty_session.json");
        std::fs::write(&session_file, b"").unwrap();
        assert!(
            session_file.exists(),
            "pre-condition: empty file must exist"
        );

        let settings = make_test_settings(&mock_server.uri(), session_file);
        let result = try_restore_session(client, &settings).await;

        assert!(
            result.is_err(),
            "expected Err for empty session file, got Ok"
        );
    }
}
