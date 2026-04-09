use std::{
    collections::BTreeMap,
    env,
    fs::{create_dir_all, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use matrix_sdk::{
    authentication::matrix::MatrixSession,
    ruma::{OwnedDeviceId, OwnedUserId, UserId},
    ServerName,
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::base::MCRSError;
use crate::cli::Args;

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Error reading configuration file: {0}")]
    IO(#[from] std::io::Error),
    #[error("Error loading JSON configuration file: {0}")]
    InvalidJSON(#[from] serde_json::Error),
    #[error("Cant find config in default place : {0}")]
    CantFindConfig(PathBuf),
    /// Returned when the config file exists but is empty; triggers default config generation.
    #[allow(dead_code)]
    #[error("Empty config file, please configure it : {0}")]
    EmptyFile(PathBuf),
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub user_id: OwnedUserId,
    pub device_id: OwnedDeviceId,
}

impl From<Session> for MatrixSession {
    fn from(value: Session) -> Self {
        MatrixSession {
            tokens: matrix_sdk::authentication::SessionTokens {
                access_token: value.access_token,
                refresh_token: value.refresh_token,
            },
            meta: matrix_sdk::SessionMeta {
                user_id: value.user_id,
                device_id: value.device_id,
            },
        }
    }
}

impl From<MatrixSession> for Session {
    fn from(value: MatrixSession) -> Self {
        Session {
            access_token: value.tokens.access_token,
            refresh_token: value.tokens.refresh_token,
            user_id: value.meta.user_id,
            device_id: value.meta.device_id,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ProfileConfig {
    pub user_id: OwnedUserId,
    pub homeserver: Option<Url>,
    pub default_room: String,
    pub dirs: Option<Directories>,
}

#[derive(Clone, Debug)]
pub struct DirectoryValues {
    pub cache: PathBuf,
    pub data: PathBuf,
    pub logs: PathBuf,
}

impl DirectoryValues {
    fn create_dir_all(&self) -> std::io::Result<()> {
        use std::fs::create_dir_all;

        let Self { cache, data, logs } = self;

        create_dir_all(cache)?;
        create_dir_all(data)?;
        create_dir_all(logs)?;

        Ok(())
    }
}

#[derive(Clone, Default, Deserialize, Serialize, Debug)]
pub struct Directories {
    pub cache: Option<String>,
    pub data: Option<String>,
    pub logs: Option<String>,
}

impl Directories {
    fn merge(self, other: Self) -> Self {
        Self {
            cache: self.cache.or(other.cache),
            data: self.data.or(other.data),
            logs: self.logs.or(other.logs),
        }
    }

    fn values(self) -> DirectoryValues {
        let cache = self
            .cache
            .map(|dir| {
                let dir = shellexpand::full(&dir)
                    .expect("unable to expand shell variables in dirs.cache");
                Path::new(dir.as_ref()).to_owned()
            })
            .or_else(|| {
                let mut dir = dirs::cache_dir()?;
                dir.push("mcrs");
                dir.into()
            })
            .expect("no dirs.cache value configured!");

        let data = self
            .data
            .map(|dir| {
                let dir = shellexpand::full(&dir)
                    .expect("unable to expand shell variables in dirs.cache");
                Path::new(dir.as_ref()).to_owned()
            })
            .or_else(|| {
                let mut dir = dirs::data_dir()?;
                dir.push("mcrs");
                dir.into()
            })
            .expect("no dirs.data value configured!");

        let logs = self
            .logs
            .map(|dir| {
                let dir = shellexpand::full(&dir)
                    .expect("unable to expand shell variables in dirs.cache");
                Path::new(dir.as_ref()).to_owned()
            })
            .unwrap_or_else(|| {
                let mut dir = cache.clone();
                dir.push("logs");
                dir
            });
        DirectoryValues { cache, data, logs }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct MCRSConfig {
    pub profiles: BTreeMap<String, ProfileConfig>,
    pub default_profile: Option<String>,
    pub dirs: Option<Directories>,
}

impl MCRSConfig {
    pub fn load_json(path: &Path) -> Result<Self, ConfigError> {
        let s = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&s)?;

        Ok(config)
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub session_json: PathBuf,
    pub sqlite_dir: PathBuf,
    /// Active profile name — used for display and future multi-profile CLI commands.
    #[allow(dead_code)]
    pub profile_name: String,
    pub profile: ProfileConfig,
    /// Resolved directory paths — retained for future file-operation commands.
    #[allow(dead_code)]
    pub dirs: DirectoryValues,
}

impl Settings {
    fn get_xdg_config_home() -> Option<PathBuf> {
        env::var("XDG_CONFIG_HOME").ok().map(PathBuf::from)
    }

    pub fn generate_default_config() {
        let mut profiles = BTreeMap::new();
        let m = <&ServerName>::try_from("matrix.org").unwrap();
        // let n = ServerName::from("str".to_string());
        profiles.insert(
            "default".to_string(),
            ProfileConfig {
                user_id: UserId::new(m),
                homeserver: Url::parse("https://matrix.org").ok(),
                default_room: String::from("default"),
                dirs: None,
            },
        );

        let cfg = MCRSConfig {
            profiles,
            default_profile: None,
            dirs: None,
        };
        let mut config_dir = Self::get_xdg_config_home()
            .or_else(dirs::config_dir)
            .expect("Specify config directory");
        config_dir.push("mcrs");
        let config_json = config_dir.join("config.json");
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        std::fs::write(&config_json, json).unwrap();
    }

    pub fn load(cli: &Args) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config_dir = Self::get_xdg_config_home()
            .or_else(dirs::config_dir)
            .expect("Specify config directory");
        config_dir.push("mcrs");
        let config_json = config_dir.join("config.json");

        let config = if config_json.is_file() {
            match MCRSConfig::load_json(config_json.as_path()) {
                Ok(result) => result,
                Err(err) => match err {
                    ConfigError::EmptyFile(_path_buf) => {
                        Self::generate_default_config();
                        panic!("Json is empty, was generated default json config");
                    }
                    er => {
                        return Err(Box::new(er));
                    }
                },
            }
        } else {
            create_dir_all(&config_dir).expect("Cant create config dir");
            let mut temp_p = config_dir.clone();
            temp_p.push("config.json");
            File::create(temp_p.clone()).expect("Cant create json config");
            return Err(Box::new(ConfigError::CantFindConfig(temp_p)));
        };

        let MCRSConfig {
            mut profiles,
            default_profile,
            dirs,
        } = config;

        let (profile_name, profile) = if let Some(profile) = cli.profile.clone().or(default_profile)
        {
            profiles.remove_entry(&profile).expect("No profile")
        } else if profiles.len() == 1 {
            profiles.into_iter().next().unwrap()
        } else {
            todo!()
        };

        Self::from_profile(profile_name, profile, dirs)
    }

    pub fn write_session(&self, session: MatrixSession) -> Result<(), MCRSError> {
        let file = File::create(self.session_json.as_path())?;
        let writer = BufWriter::new(file);
        let session = Session::from(session);
        serde_json::to_writer(writer, &session).map_err(MCRSError::from)?;
        Ok(())
    }

    pub fn read_session(&self, path: impl AsRef<Path>) -> Result<Session, MCRSError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let session = serde_json::from_reader(reader).map_err(MCRSError::from)?;
        Ok(session)
    }

    /// Build a [`Settings`] from a known profile and optional global dirs.
    /// Creates all necessary directories on disk.
    fn from_profile(
        profile_name: String,
        mut profile: ProfileConfig,
        global_dirs: Option<Directories>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let global = global_dirs.unwrap_or_default();
        let merged = profile.dirs.take().unwrap_or_default().merge(global);
        let dirs = merged.values();
        dirs.create_dir_all()?;

        let profile_data_dir = dirs.data.join("profiles").join(&profile_name);
        std::fs::create_dir_all(&profile_data_dir)?;

        Ok(Settings {
            session_json: profile_data_dir.join("session.json"),
            sqlite_dir: profile_data_dir.join("sqlite"),
            profile_name,
            profile,
            dirs,
        })
    }

    /// Try to load the settings for a specific profile.
    ///
    /// Returns `None` if the config file does not exist or the profile is not
    /// present — this is intentional so the caller can treat it as "first time
    /// login" without needing to handle an error.
    pub fn try_load_profile(profile_name: &str) -> Option<Self> {
        let config_dir = Self::get_xdg_config_home()
            .or_else(dirs::config_dir)?
            .join("mcrs");
        let config_json = config_dir.join("config.json");

        if !config_json.is_file() {
            return None;
        }

        let MCRSConfig {
            mut profiles, dirs, ..
        } = MCRSConfig::load_json(&config_json).ok()?;

        let (name, profile) = profiles.remove_entry(profile_name)?;
        Self::from_profile(name, profile, dirs).ok()
    }

    /// Create a new profile or update an existing one, persist the config to
    /// disk, and return a ready-to-use [`Settings`] for that profile.
    ///
    /// This is the entry-point for the `--login` flow: the caller has already
    /// collected homeserver / user_id / default_room and passes them in as a
    /// `ProfileConfig`.  The method guarantees that the config file and all
    /// necessary data directories exist after a successful return.
    pub fn create_or_update_profile(
        profile_name: &str,
        profile: ProfileConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // ── 1. Locate / create the config directory ─────────────────────────
        let config_dir = Self::get_xdg_config_home()
            .or_else(dirs::config_dir)
            .expect("Cannot determine config directory")
            .join("mcrs");
        std::fs::create_dir_all(&config_dir)?;

        let config_json = config_dir.join("config.json");

        // ── 2. Load existing config or start with an empty one ───────────────
        let mut config: MCRSConfig = if config_json.is_file() {
            // Tolerate a missing / corrupt file — start fresh rather than abort.
            MCRSConfig::load_json(&config_json).unwrap_or(MCRSConfig {
                profiles: BTreeMap::new(),
                default_profile: None,
                dirs: None,
            })
        } else {
            MCRSConfig {
                profiles: BTreeMap::new(),
                default_profile: None,
                dirs: None,
            }
        };

        // ── 3. Insert / replace the profile ──────────────────────────────────
        config
            .profiles
            .insert(profile_name.to_string(), profile.clone());

        // ── 4. Promote to default if none is set yet ─────────────────────────
        if config.default_profile.is_none() {
            config.default_profile = Some(profile_name.to_string());
        }

        // ── 5. Persist ────────────────────────────────────────────────────────
        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_json, &json)?;

        // ── 6. Build and return Settings ──────────────────────────────────────
        Self::from_profile(profile_name.to_string(), profile, config.dirs)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::{
        authentication::{matrix::MatrixSession, SessionTokens},
        SessionMeta,
    };
    use std::path::{Path, PathBuf};
    use tempfile::{NamedTempFile, TempDir};

    // ─── helpers ──────────────────────────────────────────────────────────────

    /// Build a `MatrixSession` with caller-supplied values so assertions are
    /// always made against known, hard-coded strings.
    fn make_matrix_session(
        access_token: &str,
        refresh_token: Option<&str>,
        user_id: &str,
        device_id: &str,
    ) -> MatrixSession {
        MatrixSession {
            tokens: SessionTokens {
                access_token: access_token.to_string(),
                refresh_token: refresh_token.map(str::to_string),
            },
            meta: SessionMeta {
                user_id: user_id.parse().expect("invalid user_id in test helper"),
                device_id: device_id.into(),
            },
        }
    }

    /// Build a minimal `Settings` whose `session_json` points to `session_path`.
    /// The other paths are placeholders – they are never accessed by the tests
    /// that use this helper.
    fn make_test_settings(session_path: PathBuf) -> Settings {
        Settings {
            session_json: session_path,
            sqlite_dir: PathBuf::from("/tmp/test_sqlite"),
            profile_name: "test".to_string(),
            profile: ProfileConfig {
                user_id: "@testuser:matrix.org".parse().unwrap(),
                homeserver: Some("https://matrix.org/".parse().unwrap()),
                default_room: "!testroom:matrix.org".to_string(),
                dirs: None,
            },
            dirs: DirectoryValues {
                cache: PathBuf::from("/tmp/test_cache"),
                data: PathBuf::from("/tmp/test_data"),
                logs: PathBuf::from("/tmp/test_logs"),
            },
        }
    }

    // ─── Session conversion tests ─────────────────────────────────────────────

    /// `MatrixSession → Session → MatrixSession` preserves every field.
    #[test]
    fn test_session_converts_from_matrix_session_round_trip() {
        let original = make_matrix_session(
            "access_token_abc",
            None,
            "@alice:matrix.org",
            "ALICE_DEVICE",
        );

        let session = Session::from(original);
        let restored = MatrixSession::from(session);

        assert_eq!(restored.tokens.access_token, "access_token_abc");
        assert_eq!(restored.tokens.refresh_token, None);
        assert_eq!(restored.meta.user_id.to_string(), "@alice:matrix.org");
        assert_eq!(restored.meta.device_id.to_string(), "ALICE_DEVICE");
    }

    /// `refresh_token` survives the full conversion cycle.
    #[test]
    fn test_session_preserves_refresh_token() {
        let original = make_matrix_session(
            "access",
            Some("my_refresh_token"),
            "@user:example.org",
            "DEV01",
        );

        let session = Session::from(original);
        let restored = MatrixSession::from(session);

        assert_eq!(
            restored.tokens.refresh_token,
            Some("my_refresh_token".to_string())
        );
    }

    /// `Session` serialises to JSON and deserialises back to an identical value
    /// (tests the `Serialize` / `Deserialize` / `PartialEq` implementations).
    #[test]
    fn test_session_json_serialization_round_trip() {
        let ms = make_matrix_session(
            "json_access_token",
            Some("json_refresh_token"),
            "@testuser:matrix.org",
            "TESTDEV",
        );
        let session = Session::from(ms);

        let json = serde_json::to_string(&session).expect("serialization failed");
        let deserialized: Session = serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(session, deserialized);
    }

    // ─── Session file I/O tests ───────────────────────────────────────────────

    /// `write_session` followed by `read_session` returns the original data.
    #[test]
    fn test_write_and_read_session_round_trip() {
        let tmpfile = NamedTempFile::new().unwrap();
        let settings = make_test_settings(tmpfile.path().to_path_buf());

        let ms = make_matrix_session(
            "stored_access_token",
            Some("stored_refresh_token"),
            "@stored_user:matrix.org",
            "STORED_DEVICE",
        );
        settings.write_session(ms).unwrap();

        let read = settings.read_session(tmpfile.path()).unwrap();
        let restored = MatrixSession::from(read);

        assert_eq!(restored.tokens.access_token, "stored_access_token");
        assert_eq!(
            restored.tokens.refresh_token,
            Some("stored_refresh_token".to_string())
        );
        assert_eq!(restored.meta.user_id.to_string(), "@stored_user:matrix.org");
        assert_eq!(restored.meta.device_id.to_string(), "STORED_DEVICE");
    }

    /// Writing a second session to the same path overwrites the first one.
    #[test]
    fn test_write_session_overwrites_previous_session() {
        let tmpfile = NamedTempFile::new().unwrap();
        let settings = make_test_settings(tmpfile.path().to_path_buf());

        let ms_v1 = make_matrix_session("token_v1", None, "@user:matrix.org", "DEV_V1");
        settings.write_session(ms_v1).unwrap();

        let ms_v2 = make_matrix_session("token_v2", None, "@user:matrix.org", "DEV_V2");
        settings.write_session(ms_v2).unwrap();

        let restored = MatrixSession::from(settings.read_session(tmpfile.path()).unwrap());
        assert_eq!(restored.tokens.access_token, "token_v2");
        assert_eq!(restored.meta.device_id.to_string(), "DEV_V2");
    }

    /// `read_session` on a file with corrupted JSON returns an error.
    #[test]
    fn test_read_session_invalid_json_returns_error() {
        let tmpfile = NamedTempFile::new().unwrap();
        std::fs::write(tmpfile.path(), b"{ not valid json }").unwrap();

        let settings = make_test_settings(tmpfile.path().to_path_buf());
        let result = settings.read_session(tmpfile.path());

        assert!(result.is_err(), "expected Err for invalid JSON, got Ok");
    }

    /// `read_session` on a path that does not exist returns an IO error.
    #[test]
    fn test_read_session_nonexistent_file_returns_error() {
        let settings = make_test_settings(PathBuf::from("/tmp/no_such_session_mcrs_test.json"));
        let result = settings.read_session("/tmp/no_such_session_mcrs_test.json");

        assert!(
            result.is_err(),
            "expected IO error for missing file, got Ok"
        );
    }

    // ─── MCRSConfig JSON parsing tests ───────────────────────────────────────

    /// A config with a single profile is parsed correctly; `default_profile` is None.
    #[test]
    fn test_config_parse_single_profile() {
        let json = r#"{
            "profiles": {
                "default": {
                    "user_id": "@user:matrix.org",
                    "homeserver": "https://matrix.org/",
                    "default_room": "!room:matrix.org"
                }
            }
        }"#;

        let tmpfile = NamedTempFile::new().unwrap();
        std::fs::write(tmpfile.path(), json).unwrap();

        let config = MCRSConfig::load_json(tmpfile.path()).unwrap();

        assert_eq!(config.profiles.len(), 1);
        assert!(config.profiles.contains_key("default"));
        assert!(config.default_profile.is_none());
    }

    /// A config with three profiles is fully parsed and all keys are present.
    #[test]
    fn test_config_parse_multiple_profiles() {
        let json = r#"{
            "profiles": {
                "alice": {
                    "user_id": "@alice:matrix.org",
                    "homeserver": "https://matrix.org/",
                    "default_room": "!alice_room:matrix.org"
                },
                "bob": {
                    "user_id": "@bob:example.org",
                    "homeserver": "https://example.org/",
                    "default_room": "!bob_room:example.org"
                },
                "charlie": {
                    "user_id": "@charlie:chat.example.com",
                    "homeserver": "https://chat.example.com/",
                    "default_room": "!charlie_room:chat.example.com"
                }
            },
            "default_profile": "alice"
        }"#;

        let tmpfile = NamedTempFile::new().unwrap();
        std::fs::write(tmpfile.path(), json).unwrap();

        let config = MCRSConfig::load_json(tmpfile.path()).unwrap();

        assert_eq!(config.profiles.len(), 3);
        assert!(config.profiles.contains_key("alice"));
        assert!(config.profiles.contains_key("bob"));
        assert!(config.profiles.contains_key("charlie"));
        assert_eq!(config.default_profile, Some("alice".to_string()));
    }

    /// `default_profile` is read from JSON and the named profile can be looked up.
    #[test]
    fn test_config_default_profile_selects_correct_entry() {
        let json = r#"{
            "profiles": {
                "work": {
                    "user_id": "@me:work.example.com",
                    "homeserver": "https://work.example.com/",
                    "default_room": "!work:work.example.com"
                },
                "personal": {
                    "user_id": "@me:matrix.org",
                    "homeserver": "https://matrix.org/",
                    "default_room": "!personal:matrix.org"
                }
            },
            "default_profile": "personal"
        }"#;

        let tmpfile = NamedTempFile::new().unwrap();
        std::fs::write(tmpfile.path(), json).unwrap();

        let config = MCRSConfig::load_json(tmpfile.path()).unwrap();

        let default_name = config.default_profile.unwrap();
        assert_eq!(default_name, "personal");

        let default_profile = config.profiles.get(&default_name).unwrap();
        assert_eq!(default_profile.user_id.to_string(), "@me:matrix.org");
    }

    /// When `default_profile` is absent from JSON it deserialises to `None`.
    #[test]
    fn test_config_no_default_profile_is_none() {
        let json = r#"{
            "profiles": {
                "only_one": {
                    "user_id": "@user:matrix.org",
                    "homeserver": "https://matrix.org/",
                    "default_room": "!room:matrix.org"
                }
            }
        }"#;

        let tmpfile = NamedTempFile::new().unwrap();
        std::fs::write(tmpfile.path(), json).unwrap();

        let config = MCRSConfig::load_json(tmpfile.path()).unwrap();

        assert!(config.default_profile.is_none());
    }

    /// Each profile contains the correct `user_id` and `homeserver`.
    #[test]
    fn test_config_profile_fields_are_correct() {
        let json = r#"{
            "profiles": {
                "alice": {
                    "user_id": "@alice:matrix.org",
                    "homeserver": "https://matrix.org/",
                    "default_room": "!room:matrix.org"
                },
                "bob": {
                    "user_id": "@bob:example.org",
                    "homeserver": "https://example.org/",
                    "default_room": "!room:example.org"
                }
            },
            "default_profile": "alice"
        }"#;

        let tmpfile = NamedTempFile::new().unwrap();
        std::fs::write(tmpfile.path(), json).unwrap();

        let config = MCRSConfig::load_json(tmpfile.path()).unwrap();
        let alice = config.profiles.get("alice").unwrap();
        let bob = config.profiles.get("bob").unwrap();

        assert_eq!(alice.user_id.to_string(), "@alice:matrix.org");
        assert_eq!(bob.user_id.to_string(), "@bob:example.org");

        assert!(alice
            .homeserver
            .as_ref()
            .unwrap()
            .as_str()
            .contains("matrix.org"));
        assert!(bob
            .homeserver
            .as_ref()
            .unwrap()
            .as_str()
            .contains("example.org"));
    }

    /// Profiles with different homeservers keep their individual URLs intact.
    #[test]
    fn test_config_profiles_can_have_different_homeservers() {
        let json = r#"{
            "profiles": {
                "work": {
                    "user_id": "@me:work.corp",
                    "homeserver": "https://matrix.work.corp/",
                    "default_room": "!work:work.corp"
                },
                "personal": {
                    "user_id": "@me:matrix.org",
                    "homeserver": "https://matrix.org/",
                    "default_room": "!personal:matrix.org"
                }
            }
        }"#;

        let tmpfile = NamedTempFile::new().unwrap();
        std::fs::write(tmpfile.path(), json).unwrap();

        let config = MCRSConfig::load_json(tmpfile.path()).unwrap();

        let work_hs = config.profiles["work"]
            .homeserver
            .as_ref()
            .unwrap()
            .as_str()
            .to_string();
        let personal_hs = config.profiles["personal"]
            .homeserver
            .as_ref()
            .unwrap()
            .as_str()
            .to_string();

        assert_ne!(work_hs, personal_hs);
        assert!(work_hs.contains("work.corp"));
        assert!(personal_hs.contains("matrix.org"));
    }

    /// An empty `profiles` map is valid and parses without error.
    #[test]
    fn test_config_empty_profiles_list() {
        let json = r#"{"profiles": {}}"#;

        let tmpfile = NamedTempFile::new().unwrap();
        std::fs::write(tmpfile.path(), json).unwrap();

        let config = MCRSConfig::load_json(tmpfile.path()).unwrap();

        assert_eq!(config.profiles.len(), 0);
        assert!(config.default_profile.is_none());
    }

    /// Malformed JSON produces `ConfigError::InvalidJSON`.
    #[test]
    fn test_config_invalid_json_returns_parse_error() {
        let tmpfile = NamedTempFile::new().unwrap();
        std::fs::write(tmpfile.path(), b"{ not: valid_json }").unwrap();

        let result = MCRSConfig::load_json(tmpfile.path());

        assert!(
            matches!(result, Err(ConfigError::InvalidJSON(_))),
            "expected ConfigError::InvalidJSON, got {:?}",
            result.err()
        );
    }

    /// A missing file produces `ConfigError::IO`.
    #[test]
    fn test_config_nonexistent_file_returns_io_error() {
        let result = MCRSConfig::load_json(Path::new(
            "/tmp/definitely_does_not_exist_mcrs_test_config.json",
        ));

        assert!(
            matches!(result, Err(ConfigError::IO(_))),
            "expected ConfigError::IO, got {:?}",
            result.err()
        );
    }

    // ─── Multi-account isolation tests ────────────────────────────────────────

    /// Each profile stores its session in a distinct path; sessions don't bleed
    /// between accounts.
    #[test]
    fn test_multiple_accounts_have_isolated_session_files() {
        let tmpdir = TempDir::new().unwrap();

        // Reproduce the per-profile directory layout that `Settings::load` builds.
        let alice_dir = tmpdir.path().join("profiles/alice");
        let bob_dir = tmpdir.path().join("profiles/bob");
        std::fs::create_dir_all(&alice_dir).unwrap();
        std::fs::create_dir_all(&bob_dir).unwrap();

        let alice_session_path = alice_dir.join("session.json");
        let bob_session_path = bob_dir.join("session.json");

        // The two paths must be distinct.
        assert_ne!(alice_session_path, bob_session_path);

        let settings_alice = Settings {
            session_json: alice_session_path.clone(),
            sqlite_dir: alice_dir.join("sqlite"),
            profile_name: "alice".to_string(),
            profile: ProfileConfig {
                user_id: "@alice:matrix.org".parse().unwrap(),
                homeserver: Some("https://matrix.org/".parse().unwrap()),
                default_room: "!alice_room:matrix.org".to_string(),
                dirs: None,
            },
            dirs: DirectoryValues {
                cache: tmpdir.path().join("cache/alice"),
                data: tmpdir.path().join("data/alice"),
                logs: tmpdir.path().join("logs/alice"),
            },
        };

        let settings_bob = Settings {
            session_json: bob_session_path.clone(),
            sqlite_dir: bob_dir.join("sqlite"),
            profile_name: "bob".to_string(),
            profile: ProfileConfig {
                user_id: "@bob:example.org".parse().unwrap(),
                homeserver: Some("https://example.org/".parse().unwrap()),
                default_room: "!bob_room:example.org".to_string(),
                dirs: None,
            },
            dirs: DirectoryValues {
                cache: tmpdir.path().join("cache/bob"),
                data: tmpdir.path().join("data/bob"),
                logs: tmpdir.path().join("logs/bob"),
            },
        };

        // Write distinct sessions for each profile.
        settings_alice
            .write_session(make_matrix_session(
                "alice_token",
                None,
                "@alice:matrix.org",
                "ALICE_DEV",
            ))
            .unwrap();
        settings_bob
            .write_session(make_matrix_session(
                "bob_token",
                Some("bob_refresh"),
                "@bob:example.org",
                "BOB_DEV",
            ))
            .unwrap();

        // Alice's session is unaffected by Bob's write.
        let alice_restored =
            MatrixSession::from(settings_alice.read_session(&alice_session_path).unwrap());
        assert_eq!(alice_restored.tokens.access_token, "alice_token");
        assert_eq!(alice_restored.tokens.refresh_token, None);
        assert_eq!(alice_restored.meta.user_id.to_string(), "@alice:matrix.org");
        assert_eq!(alice_restored.meta.device_id.to_string(), "ALICE_DEV");

        // Bob's session is independent of Alice's.
        let bob_restored =
            MatrixSession::from(settings_bob.read_session(&bob_session_path).unwrap());
        assert_eq!(bob_restored.tokens.access_token, "bob_token");
        assert_eq!(
            bob_restored.tokens.refresh_token,
            Some("bob_refresh".to_string())
        );
        assert_eq!(bob_restored.meta.user_id.to_string(), "@bob:example.org");
        assert_eq!(bob_restored.meta.device_id.to_string(), "BOB_DEV");
    }

    /// Refreshing one account's session does not corrupt another account's data.
    #[test]
    fn test_overwrite_one_account_session_does_not_affect_other() {
        let tmpdir = TempDir::new().unwrap();
        let alice_path = tmpdir.path().join("alice_session.json");
        let bob_path = tmpdir.path().join("bob_session.json");

        let settings_alice = make_test_settings(alice_path.clone());
        let settings_bob = Settings {
            session_json: bob_path.clone(),
            sqlite_dir: PathBuf::from("/tmp/bob_sqlite"),
            profile_name: "bob".to_string(),
            profile: ProfileConfig {
                user_id: "@bob:example.org".parse().unwrap(),
                homeserver: Some("https://example.org/".parse().unwrap()),
                default_room: "!bob:example.org".to_string(),
                dirs: None,
            },
            dirs: DirectoryValues {
                cache: PathBuf::from("/tmp/bob_cache"),
                data: PathBuf::from("/tmp/bob_data"),
                logs: PathBuf::from("/tmp/bob_logs"),
            },
        };

        // Initial write for both accounts.
        settings_alice
            .write_session(make_matrix_session(
                "alice_v1",
                None,
                "@alice:matrix.org",
                "ALICE",
            ))
            .unwrap();
        settings_bob
            .write_session(make_matrix_session(
                "bob_token",
                None,
                "@bob:example.org",
                "BOB",
            ))
            .unwrap();

        // Overwrite Alice's session with a renewed token.
        settings_alice
            .write_session(make_matrix_session(
                "alice_v2",
                None,
                "@alice:matrix.org",
                "ALICE2",
            ))
            .unwrap();

        // Alice now has the new token.
        let alice_restored = MatrixSession::from(settings_alice.read_session(&alice_path).unwrap());
        assert_eq!(alice_restored.tokens.access_token, "alice_v2");
        assert_eq!(alice_restored.meta.device_id.to_string(), "ALICE2");

        // Bob's session remains completely untouched.
        let bob_restored = MatrixSession::from(settings_bob.read_session(&bob_path).unwrap());
        assert_eq!(bob_restored.tokens.access_token, "bob_token");
        assert_eq!(bob_restored.meta.device_id.to_string(), "BOB");
    }

    // ─── Directories merge tests ──────────────────────────────────────────────

    /// Fields in `self` (profile-level) shadow matching fields from `other`
    /// (global-level).
    #[test]
    fn test_directories_merge_self_takes_priority_over_other() {
        let profile_dirs = Directories {
            cache: Some("/profile/cache".to_string()),
            data: None,
            logs: Some("/profile/logs".to_string()),
        };
        let global_dirs = Directories {
            cache: Some("/global/cache".to_string()),
            data: Some("/global/data".to_string()),
            logs: Some("/global/logs".to_string()),
        };

        let merged = profile_dirs.merge(global_dirs);

        // profile values win for cache and logs.
        assert_eq!(merged.cache, Some("/profile/cache".to_string()));
        assert_eq!(merged.logs, Some("/profile/logs".to_string()));
        // global value used for data because profile had None.
        assert_eq!(merged.data, Some("/global/data".to_string()));
    }

    /// When all profile fields are `None`, every field falls back to the global
    /// value.
    #[test]
    fn test_directories_merge_falls_back_to_other_when_all_none() {
        let merged = Directories {
            cache: None,
            data: None,
            logs: None,
        }
        .merge(Directories {
            cache: Some("/global/cache".to_string()),
            data: Some("/global/data".to_string()),
            logs: Some("/global/logs".to_string()),
        });

        assert_eq!(merged.cache, Some("/global/cache".to_string()));
        assert_eq!(merged.data, Some("/global/data".to_string()));
        assert_eq!(merged.logs, Some("/global/logs".to_string()));
    }

    /// Merging two `Default` (all-`None`) structs yields another all-`None`
    /// struct.
    #[test]
    fn test_directories_merge_both_none_stays_none() {
        let merged = Directories::default().merge(Directories::default());

        assert!(merged.cache.is_none());
        assert!(merged.data.is_none());
        assert!(merged.logs.is_none());
    }

    /// When both sides are fully specified, every field comes from `self`.
    #[test]
    fn test_directories_merge_both_set_self_wins_for_every_field() {
        let merged = Directories {
            cache: Some("/p/cache".to_string()),
            data: Some("/p/data".to_string()),
            logs: Some("/p/logs".to_string()),
        }
        .merge(Directories {
            cache: Some("/g/cache".to_string()),
            data: Some("/g/data".to_string()),
            logs: Some("/g/logs".to_string()),
        });

        assert_eq!(merged.cache, Some("/p/cache".to_string()));
        assert_eq!(merged.data, Some("/p/data".to_string()));
        assert_eq!(merged.logs, Some("/p/logs".to_string()));
    }

    // ─── XDG env / Settings::load / generate_default_config tests ────────────

    use crate::cli::Args;
    use clap::Parser;

    /// Mutex to serialize tests that mutate the `XDG_CONFIG_HOME` environment
    /// variable so they don't interfere with one another when run in parallel.
    static XDG_ENV_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

    /// Write `content` to `<xdg_dir>/mcrs/config.json`, creating parent dirs.
    fn write_xdg_config(xdg_dir: &Path, content: &str) {
        let mcrs = xdg_dir.join("mcrs");
        std::fs::create_dir_all(&mcrs).unwrap();
        std::fs::write(mcrs.join("config.json"), content).unwrap();
    }

    /// Build a JSON object for one profile entry that sets explicit `dirs`
    /// pointing inside `tmpdir`, so `Settings::load()` never touches the real
    /// user home during tests.
    fn profile_entry_json(user_id: &str, homeserver: &str, room: &str, tmpdir: &Path) -> String {
        let data = tmpdir.join("data").to_str().unwrap().to_string();
        let cache = tmpdir.join("cache").to_str().unwrap().to_string();
        format!(
            r#"{{"user_id":"{user_id}","homeserver":"{homeserver}","default_room":"{room}","dirs":{{"data":"{data}","cache":"{cache}"}}}}"#
        )
    }

    /// When no config file exists yet, `Settings::load()` must return `Err` AND
    /// create the empty `mcrs/config.json` placeholder so the user knows where
    /// to put their configuration.
    #[test]
    fn test_load_returns_error_and_creates_file_when_config_missing() {
        let _guard = XDG_ENV_LOCK.lock().unwrap();
        let tmpdir = TempDir::new().unwrap();
        #[allow(deprecated)]
        std::env::set_var("XDG_CONFIG_HOME", tmpdir.path());

        let result = Settings::load(&Args::try_parse_from(["mcrs"]).unwrap());

        assert!(result.is_err(), "Expected Err when config file is absent");
        assert!(
            tmpdir.path().join("mcrs").join("config.json").exists(),
            "Expected config.json to be created by load()"
        );

        #[allow(deprecated)]
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    /// An empty config file (zero bytes) is not valid JSON; `Settings::load()`
    /// must return `Err` rather than panic or silently succeed.
    #[test]
    fn test_load_returns_error_for_empty_config_file() {
        let _guard = XDG_ENV_LOCK.lock().unwrap();
        let tmpdir = TempDir::new().unwrap();
        #[allow(deprecated)]
        std::env::set_var("XDG_CONFIG_HOME", tmpdir.path());
        write_xdg_config(tmpdir.path(), "");

        let result = Settings::load(&Args::try_parse_from(["mcrs"]).unwrap());

        assert!(result.is_err(), "Expected Err for empty config file");

        #[allow(deprecated)]
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    /// When exactly one profile is present and no `--profile` flag is given,
    /// `Settings::load()` should auto-select that profile.
    #[test]
    fn test_load_auto_selects_single_profile() {
        let _guard = XDG_ENV_LOCK.lock().unwrap();
        let tmpdir = TempDir::new().unwrap();
        #[allow(deprecated)]
        std::env::set_var("XDG_CONFIG_HOME", tmpdir.path());

        let entry = profile_entry_json(
            "@alice:matrix.org",
            "https://matrix.org/",
            "!room:matrix.org",
            tmpdir.path(),
        );
        let config = format!(r#"{{"profiles":{{"alice":{entry}}}}}"#);
        write_xdg_config(tmpdir.path(), &config);

        let result = Settings::load(&Args::try_parse_from(["mcrs"]).unwrap());

        assert!(result.is_ok(), "Expected Ok for single-profile config");
        let settings = result.unwrap();
        assert_eq!(settings.profile_name, "alice");
        assert_eq!(settings.profile.user_id.to_string(), "@alice:matrix.org");

        #[allow(deprecated)]
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    /// When `default_profile` is set in the config and no `--profile` flag is
    /// passed, `Settings::load()` picks that profile.
    #[test]
    fn test_load_selects_default_profile() {
        let _guard = XDG_ENV_LOCK.lock().unwrap();
        let tmpdir = TempDir::new().unwrap();
        #[allow(deprecated)]
        std::env::set_var("XDG_CONFIG_HOME", tmpdir.path());

        let alice = profile_entry_json(
            "@alice:matrix.org",
            "https://matrix.org/",
            "!room:matrix.org",
            tmpdir.path(),
        );
        let bob = profile_entry_json(
            "@bob:example.org",
            "https://example.org/",
            "!room:example.org",
            tmpdir.path(),
        );
        let config =
            format!(r#"{{"profiles":{{"alice":{alice},"bob":{bob}}},"default_profile":"bob"}}"#);
        write_xdg_config(tmpdir.path(), &config);

        let result = Settings::load(&Args::try_parse_from(["mcrs"]).unwrap());

        assert!(result.is_ok(), "Expected Ok when default_profile is set");
        let settings = result.unwrap();
        assert_eq!(settings.profile_name, "bob");
        assert_eq!(settings.profile.user_id.to_string(), "@bob:example.org");

        #[allow(deprecated)]
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    /// The `--profile` CLI flag must override whatever `default_profile` says
    /// in the config file.
    #[test]
    fn test_load_profile_flag_overrides_default_profile() {
        let _guard = XDG_ENV_LOCK.lock().unwrap();
        let tmpdir = TempDir::new().unwrap();
        #[allow(deprecated)]
        std::env::set_var("XDG_CONFIG_HOME", tmpdir.path());

        let alice = profile_entry_json(
            "@alice:matrix.org",
            "https://matrix.org/",
            "!room:matrix.org",
            tmpdir.path(),
        );
        let bob = profile_entry_json(
            "@bob:example.org",
            "https://example.org/",
            "!room:example.org",
            tmpdir.path(),
        );
        let config =
            format!(r#"{{"profiles":{{"alice":{alice},"bob":{bob}}},"default_profile":"bob"}}"#);
        write_xdg_config(tmpdir.path(), &config);

        let result = Settings::load(&Args::try_parse_from(["mcrs", "--profile", "alice"]).unwrap());

        assert!(result.is_ok(), "Expected Ok when --profile flag is used");
        let settings = result.unwrap();
        assert_eq!(settings.profile_name, "alice");

        #[allow(deprecated)]
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    /// `session_json` and `sqlite_dir` must be rooted under the profile-specific
    /// sub-directory `profiles/<profile_name>/` inside the data directory.
    #[test]
    fn test_load_constructs_session_json_path_under_profile_name() {
        let _guard = XDG_ENV_LOCK.lock().unwrap();
        let tmpdir = TempDir::new().unwrap();
        #[allow(deprecated)]
        std::env::set_var("XDG_CONFIG_HOME", tmpdir.path());

        let entry = profile_entry_json(
            "@user:matrix.org",
            "https://matrix.org/",
            "!room:matrix.org",
            tmpdir.path(),
        );
        let config = format!(r#"{{"profiles":{{"myprofile":{entry}}}}}"#);
        write_xdg_config(tmpdir.path(), &config);

        let result = Settings::load(&Args::try_parse_from(["mcrs"]).unwrap());

        assert!(result.is_ok(), "Expected Ok for myprofile");
        let settings = result.unwrap();
        assert!(
            settings
                .session_json
                .ends_with("profiles/myprofile/session.json"),
            "session_json should end with profiles/myprofile/session.json, got {:?}",
            settings.session_json
        );
        assert!(
            settings.sqlite_dir.ends_with("profiles/myprofile/sqlite"),
            "sqlite_dir should end with profiles/myprofile/sqlite, got {:?}",
            settings.sqlite_dir
        );

        #[allow(deprecated)]
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    /// `generate_default_config()` must write a well-formed JSON file that
    /// contains at least a `"default"` profile entry with a non-empty `user_id`.
    #[test]
    fn test_generate_default_config_creates_valid_json_with_default_profile() {
        let _guard = XDG_ENV_LOCK.lock().unwrap();
        let tmpdir = TempDir::new().unwrap();
        #[allow(deprecated)]
        std::env::set_var("XDG_CONFIG_HOME", tmpdir.path());
        // generate_default_config() uses std::fs::write which requires the
        // parent directory to already exist — it does not create mcrs/ itself.
        std::fs::create_dir_all(tmpdir.path().join("mcrs")).unwrap();

        Settings::generate_default_config();

        let config_path = tmpdir.path().join("mcrs").join("config.json");
        assert!(config_path.exists(), "config.json should have been created");

        let content = std::fs::read_to_string(&config_path).unwrap();
        let config: MCRSConfig =
            serde_json::from_str(&content).expect("config.json must be valid JSON");

        assert!(
            config.profiles.contains_key("default"),
            "Generated config should contain a 'default' profile"
        );
        let default_profile = &config.profiles["default"];
        assert!(
            !default_profile.user_id.to_string().is_empty(),
            "Default profile must have a non-empty user_id"
        );

        #[allow(deprecated)]
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
