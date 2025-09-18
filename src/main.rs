//! Welcome to the matrix-commander crate!
//!
//! Please help create the Rust version of matrix-commander.
//! Please consider providing Pull Requests.
//! Have a look at: <https://github.com/8go/matrix-commander-rs>
//!
//! `matrix-commander-rs` is a (partial initial) re-implementation
//! of the feature-rich `matrix-commander` (Python) program with
//! its repo at <https://github.com/8go/matrix-commander>.
//!
//! matrix-commander is a simple terminal-based CLI client of
//! Matrix <https://matrix.org>. It let's you login to your
//! Matrix account, verify your new devices, and send encrypted
//! (or not-encrypted) messages and files on the Matrix network.
//!
//!
//! Please help improve the code and add features  :pray:  :clap:
//!
//! Usage:
//! - matrix-commander-rs --login password # first time only
//! - matrix-commander-rs --bootstrap --verify manual-device # manual verification
//! - matrix-commander-rs --verify emoji # emoji verification
//! - matrix-commander-rs --message "Hello World" "Good Bye!"
//! - matrix-commander-rs --file test.txt
//! - or do many things at a time:
//! - matrix-commander-rs --login password --verify manual-device
//! - matrix-commander-rs --message Hi --file test.txt --devices --get-room-info
//!
//! For more information, see the README.md
//! <https://github.com/8go/matrix-commander-rs/blob/main/README.md>
//! file.

use crate::settings::{ProfileConfig, Settings};
use clap::{CommandFactory, Parser, ValueEnum};
use colored::Colorize;

use matrix_sdk::ruma::api::client::room::Visibility;
use regex::Regex;
use rpassword::read_password;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::io::{self, stdin, stdout, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, process};
use thiserror::Error;
use tracing::{debug, enabled, error, info, warn, Level};
use tracing_subscriber::EnvFilter;
use update_informer::{registry, Check};
use url::Url;

use matrix_sdk::encryption::{BackupDownloadStrategy, EncryptionSettings};
use matrix_sdk::{
    // config::{RequestConfig, StoreConfig, SyncSettings},
    // instant::Duration,
    // room,
    ruma::OwnedUserId,
    Client,
};
use serde_json::json;

/// import matrix-sdk Client related code of general kind: logout, verify, sync, etc
mod mclient;
use crate::cli::Args;
use crate::mclient::{
    bootstrap, convert_to_full_alias_ids, convert_to_full_mxc_uris, convert_to_full_room_id,
    convert_to_full_room_ids, convert_to_full_user_ids, delete_devices_pre, devices, file,
    get_avatar, get_avatar_url, get_display_name, get_masterkey, get_profile, get_room_info,
    invited_rooms, joined_members, joined_rooms, left_rooms, logout, media_delete, media_download,
    media_mxc_to_http, media_upload, message, replace_star_with_rooms, room_ban, room_create,
    room_enable_encryption, room_forget, room_get_state, room_get_visibility, room_invite,
    room_join, room_kick, room_leave, room_resolve_alias, room_unban, rooms, set_avatar,
    set_avatar_url, set_display_name, sync_once, unset_avatar_url, verify, MessageOptions,
};

// import matrix-sdk Client related code related to receiving messages and listening
mod listen;
pub mod output;

use crate::listen::{listen_all, listen_forever, listen_once, listen_tail};
use crate::output::Output;

mod login;

mod settings;

use crate::settings::{SessionJson, SqliteStore};

mod base;
use crate::base::{
    consts::*, get_pkg_name, get_pkg_repository, get_prog_without_ext, get_store_default_path,
    get_version,
};

mod cli;

/// The enumerator for Errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Custom(&'static str),

    #[error("No valid home directory path")]
    NoHomeDirectory,

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("Invalid Room")]
    InvalidRoom,

    #[error("Homeserver Not Set")]
    HomeserverNotSet,

    #[error("Invalid File")]
    InvalidFile,

    #[error("Login Failed")]
    LoginFailed,

    #[error("Verify Failed or Partially Failed")]
    VerifyFailed,

    #[error("Bootstrap Failed")]
    BootstrapFailed,

    #[error("Login Unnecessary")]
    LoginUnnecessary,

    #[error("Send Failed")]
    SendFailed,

    #[error("Listen Failed")]
    ListenFailed,

    #[error("Create Room Failed")]
    CreateRoomFailed,

    #[error("Leave Room Failed")]
    LeaveRoomFailed,

    #[error("Forget Room Failed")]
    ForgetRoomFailed,

    #[error("Invite Room Failed")]
    InviteRoomFailed,

    #[error("Join Room Failed")]
    JoinRoomFailed,

    #[error("Ban Room Failed")]
    BanRoomFailed,

    #[error("Unban Room Failed")]
    UnbanRoomFailed,

    #[error("Kick Room Failed")]
    KickRoomFailed,

    #[error("Resolve Room Alias Failed")]
    ResolveRoomAliasFailed,

    #[error("Enable Encryption Failed")]
    EnableEncryptionFailed,

    #[error("Room Get Visibility Failed")]
    RoomGetVisibilityFailed,

    #[error("Room Get State Failed")]
    RoomGetStateFailed,

    #[error("JoinedMembersFailed")]
    JoinedMembersFailed,

    #[error("Delete Device Failed")]
    DeleteDeviceFailed,

    #[error("Get Avatar Failed")]
    GetAvatarFailed,

    #[error("Set Avatar Failed")]
    SetAvatarFailed,

    #[error("Get Avatar URL Failed")]
    GetAvatarUrlFailed,

    #[error("Set Avatar URL Failed")]
    SetAvatarUrlFailed,

    #[error("Unset Avatar URL Failed")]
    UnsetAvatarUrlFailed,

    #[error("Get Displayname Failed")]
    GetDisplaynameFailed,

    #[error("Set Displayname Failed")]
    SetDisplaynameFailed,

    #[error("Get Profile Failed")]
    GetProfileFailed,

    #[error("Get Masterkey Failed")]
    GetMasterkeyFailed,

    #[error("Restoring Login Failed")]
    RestoreLoginFailed,

    #[error("Media Upload Failed")]
    MediaUploadFailed,

    #[error("Media Download Failed")]
    MediaDownloadFailed,

    #[error("Media Delete Failed")]
    MediaDeleteFailed,

    #[error("MXC TO HTTP Failed")]
    MediaMxcToHttpFailed,

    #[error("Invalid Client Connection")]
    InvalidClientConnection,

    #[error("Unknown CLI parameter")]
    UnknownCliParameter,

    #[error("Unsupported CLI parameter: {0}")]
    UnsupportedCliParameter(&'static str),

    #[error("Missing Room")]
    MissingRoom,

    #[error("Missing User")]
    MissingUser,

    #[error("Missing Password")]
    MissingPassword,

    #[error("Missing CLI parameter")]
    MissingCliParameter,

    #[error("Not Implemented Yet")]
    NotImplementedYet,

    #[error("No Credentials Found")]
    NoCredentialsFound,

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Matrix(#[from] matrix_sdk::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Http(#[from] matrix_sdk::HttpError),
}

/// Function to create custom error messaages on the fly with static text
#[allow(dead_code)]
impl Error {
    pub(crate) fn custom<T>(message: &'static str) -> Result<T, Error> {
        Err(Error::Custom(message))
    }
}

/// Enumerator used for --sync option
#[derive(Clone, Debug, Copy, PartialEq, Default, ValueEnum)]
enum Sync {
    // None: only useful if one needs to know if option was used or not.
    // Sort of like an or instead of an Option<Sync>.
    // We do not need to know if user used the option or not,
    // we just need to know the value.
    // None,
    /// Turns syncing off for sending operations to improve performance
    Off,
    // partial,
    /// full: the default value
    #[default]
    Full,
}

/// is_ functions for the enum
impl Sync {
    pub fn is_off(&self) -> bool {
        self == &Self::Off
    }
    pub fn is_full(&self) -> bool {
        self == &Self::Full
    }
}

/// Converting from String to Sync for --sync option
impl FromStr for Sync {
    type Err = ();
    fn from_str(src: &str) -> Result<Sync, ()> {
        match src.to_lowercase().trim() {
            "off" => Ok(Sync::Off),
            "full" => Ok(Sync::Full),
            _ => Err(()),
        }
    }
}

/// Creates .to_string() for Sync for --sync option
impl fmt::Display for Sync {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

/// Enumerator used for --version option
#[derive(Clone, Debug, Copy, PartialEq, Default, ValueEnum)]
enum Version {
    /// Check if there is a newer version available
    #[default]
    Check,
}

// /// is_ functions for the enum
// impl Version {
//     pub fn is_check(&self) -> bool {
//         self == &Self::Check
//     }
// }

/// Converting from String to Version for --version option
impl FromStr for Version {
    type Err = ();
    fn from_str(src: &str) -> Result<Version, ()> {
        match src.to_lowercase().trim() {
            "check" => Ok(Version::Check),
            _ => Err(()),
        }
    }
}

/// Creates .to_string() for Sync for --sync option
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

/// Enumerator used for --verify option
#[derive(Clone, Debug, Copy, PartialEq, Default, ValueEnum)]
enum Verify {
    /// None: option not used, no verification done
    #[default]
    None,
    /// ManualDevice: manual device verification
    /// See also: https://docs.rs/matrix-sdk/0.7/matrix_sdk/encryption/identities/struct.Device.html#method.verify
    ManualDevice,
    /// ManualUser: manual user verification
    /// See also: https://docs.rs/matrix-sdk/0.7/matrix_sdk/encryption/identities/struct.UserIdentity.html#method.verify
    ManualUser,
    /// Emoji: verify via emojis as the recipient
    Emoji,
    /// Emoji: verify via emojis as the initiator
    EmojiReq,
}

/// is_ functions for the enum
impl Verify {
    pub fn is_none(&self) -> bool {
        self == &Self::None
    }
    pub fn is_manual_device(&self) -> bool {
        self == &Self::ManualDevice
    }
    pub fn is_manual_user(&self) -> bool {
        self == &Self::ManualUser
    }
    pub fn is_emoji(&self) -> bool {
        self == &Self::Emoji
    }
    pub fn is_emoji_req(&self) -> bool {
        self == &Self::EmojiReq
    }
}

/// Converting from String to Verify for --verify option
impl FromStr for Verify {
    type Err = ();
    fn from_str(src: &str) -> Result<Verify, ()> {
        match src.to_lowercase().trim() {
            "none" => Ok(Verify::None),
            "manual-device" => Ok(Verify::ManualDevice),
            "manual-user" => Ok(Verify::ManualUser),
            "emoji" => Ok(Verify::Emoji),
            "emoji-req" => Ok(Verify::EmojiReq),
            _ => Err(()),
        }
    }
}

/// Creates .to_string() for Verify for --verify option
impl fmt::Display for Verify {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

/// Enumerator used for --logout option
#[derive(Clone, Debug, Copy, PartialEq, Default, ValueEnum)]
enum Logout {
    /// None: Log out nowhere, don't do anything, default
    #[default]
    None,
    /// Me: Log out from the currently used device
    Me,
    /// All: Log out from all devices of the user
    All,
}

/// is_ functions for the enum
impl Logout {
    pub fn is_none(&self) -> bool {
        self == &Self::None
    }
    pub fn is_me(&self) -> bool {
        self == &Self::Me
    }
    pub fn is_all(&self) -> bool {
        self == &Self::All
    }
}

/// Converting from String to Logout for --logout option
impl FromStr for Logout {
    type Err = ();
    fn from_str(src: &str) -> Result<Logout, ()> {
        match src.to_lowercase().trim() {
            "none" => Ok(Logout::None),
            "me" => Ok(Logout::Me),
            "all" => Ok(Logout::All),
            _ => Err(()),
        }
    }
}

/// Creates .to_string() for Sync for --sync option
impl fmt::Display for Logout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

/// Enumerator used for --listen (--tail) option
#[derive(Clone, Debug, Copy, PartialEq, Default, ValueEnum)]
enum Listen {
    // None: only useful if one needs to know if option was used or not.
    // Sort of like an or instead of an Option<Sync>.
    // We do not need to know if user used the option or not,
    // we just need to know the value.
    /// Never: Indicates to not listen, default
    #[default]
    Never,
    /// Once: Indicates to listen once in *all* rooms and then continue
    Once,
    /// Forever: Indicates to listen forever in *all* rooms, until process is killed manually.
    /// This is the only option that remains in the event loop.
    Forever,
    /// Tail: Indicates to get the last N messages from the specified romm(s) and then continue
    Tail,
    /// All: Indicates to get *all* the messages from from the specified romm(s) and then continue
    All,
}

/// is_ functions for the enum
impl Listen {
    pub fn is_never(&self) -> bool {
        self == &Self::Never
    }
    pub fn is_once(&self) -> bool {
        self == &Self::Once
    }
    pub fn is_forever(&self) -> bool {
        self == &Self::Forever
    }
    pub fn is_tail(&self) -> bool {
        self == &Self::Tail
    }
    pub fn is_all(&self) -> bool {
        self == &Self::All
    }
}

/// Converting from String to Listen for --listen option
impl FromStr for Listen {
    type Err = ();
    fn from_str(src: &str) -> Result<Listen, ()> {
        match src.to_lowercase().trim() {
            "never" => Ok(Listen::Never),
            "once" => Ok(Listen::Once),
            "forever" => Ok(Listen::Forever),
            "tail" => Ok(Listen::Tail),
            "all" => Ok(Listen::All),
            _ => Err(()),
        }
    }
}

/// Creates .to_string() for Listen for --listen option
impl fmt::Display for Listen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

/// Enumerator used for --log-level option
#[derive(Clone, Debug, Copy, PartialEq, Default, ValueEnum)]
enum LogLevel {
    /// None: not set, default.
    #[default]
    None,
    /// Error: Indicates to print only errors
    Error,
    /// Warn: Indicates to print warnings and errors
    Warn,
    /// Info: Indicates to to print info, warn and errors
    Info,
    /// Debug: Indicates to to print debug and the rest
    Debug,
    /// Trace: Indicates to to print everything
    Trace,
}

/// is_ functions for the enum
impl LogLevel {
    pub fn is_none(&self) -> bool {
        self == &Self::None
    }
    // pub fn is_error(&self) -> bool { self == &Self::Error }
}

// No longer used, as ValueEnum from clap crate provides similar function.
// /// Converting from String to LogLevel for --log-level option
// impl FromStr for LogLevel {
//     type Err = ();
//     fn from_str(src: &str) -> Result<LogLevel, ()> {
//         return match src.to_lowercase().trim() {
//             "none" => Ok(LogLevel::None),
//             "error" => Ok(LogLevel::Error),
//             "warn" => Ok(LogLevel::Warn),
//             "info" => Ok(LogLevel::Info),
//             "debug" => Ok(LogLevel::Debug),
//             "trace" => Ok(LogLevel::Trace),
//             _ => Err(()),
//         };
//     }
// }

/// Creates .to_string() for Listen for --listen option
impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Default, ValueEnum)]
enum LoginCLI {
    #[default]
    None,
    Password,
    Sso,
}

/// Gets the *actual* path (including file name) of the store directory
/// The default path might not be the actual path as it can be overwritten with command line
/// options.
/// set_store() must be called before this function is ever called.
fn get_store_actual_path(ap: &Args) -> &PathBuf {
    &ap.store
}

/// Return true if store dir exists, false otherwise
#[allow(dead_code)]
fn store_exist(ap: &Args) -> bool {
    let dp = get_store_default_path();
    let ap = get_store_actual_path(ap);
    debug!(
        "store_default_path = {:?}, store_actual_path = {:?}",
        dp, ap
    );
    let exists = ap.is_dir();
    if exists {
        debug!(
            "{:?} exists and is directory. Not sure if readable though.",
            ap
        );
    } else {
        debug!("{:?} does not exist or is not a directory.", ap);
    }
    exists
}

/// Prints the usage info
pub fn usage() {
    let help_str = Args::command().render_usage().to_string();
    println!("{}", &help_str);
    println!("Options:");
    let help_str = Args::command().render_help().to_string();
    let v: Vec<&str> = help_str.split('\n').collect();
    for l in v {
        if l.starts_with("  -") || l.starts_with("      --") {
            println!("{}", &l);
        }
    }
}

/// Prints the short help
pub fn help() {
    let help_str = Args::command().render_help().to_string();
    // println!("{}", &help_str);
    // regex to remove shortest pieces "Details:: ... \n  -"
    // regex to remove shortest pieces "Details:: ... \n      --"
    // regex to remove shortest pieces "Details:: ... \nPS:"
    // 2 regex groups: delete and keep.
    // [\S\s]*? ... match anything in a non-greedy fashion
    // stop when either "PS:", "  -" or "      --" is reached
    let re = Regex::new(r"(?P<del>[ ]+Details::[\S\s]*?)(?P<keep>\nPS:|\n  -|\n      --)").unwrap();
    let after = re.replace_all(&help_str, "$keep");
    print!("{}", &after.replace("\n\n", "\n")); // remove empty lines
    println!("Use --manual to get more detailed help information.");
}

/// Prints the long help
pub fn manual() {
    let help_str = Args::command().render_long_help().to_string();
    println!("{}", &help_str);
}

/// Prints the README.md file
pub async fn readme() {
    match reqwest::get(URL_README).await {
        Ok(resp) => {
            debug!("Got README.md file from URL {:?}.", URL_README);
            println!("{}", resp.text().await.unwrap())
        }
        Err(ref e) => {
            println!(
                "Error getting README.md from {:#?}. Reported error {:?}.",
                URL_README, e
            );
        }
    };
}

/// Prints the version information
pub fn version(output: Output) {
    let program = get_prog_without_ext();
    let version = get_version();
    let repo = get_pkg_repository();
    match output {
        Output::Text => {
            let colored = if stdout().is_terminal() {
                version.green()
            } else {
                version.normal()
            };
            println!();
            println!("  _|      _|      _|_|_|                     {}", program);
            print!("  _|_|  _|_|    _|             _~^~^~_       ");
            println!("a rusty vision of a Matrix CLI client");
            println!(
                "  _|  _|  _|    _|         \\) /  o o  \\ (/   version {}",
                colored
            );
            println!("  _|      _|    _|           '_   -   _'     repo {}", repo);
            print!("  _|      _|      _|_|_|     / '-----' \\     ");
            println!("please submit PRs to make the vision a reality");
            println!();
        }
        Output::Json | Output::JsonMax => {
            let info = json!({
                "program" : program,
                "version" : version,
                "repo" : repo,
            });
            println!("{}", serde_json::to_string(&info).unwrap())
        }
        Output::JsonSpec => (),
    }
}

/// Prints the installed version and the latest crates.io-available version
pub fn version_check() {
    println!("Installed version: v{}", get_version());
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let informer = update_informer::new(registry::Crates, name, version).check_version();
    let avail = "New version is available";
    let uptod = "You are up-to-date.";
    let couldnot = "Could not get latest version.";
    let available;
    let uptodate;
    let couldnotget;
    if stdout().is_terminal() {
        // debug!("stdout is a terminal so we can use color codes")
        available = avail.yellow();
        uptodate = uptod.green();
        couldnotget = couldnot.red();
    } else {
        available = avail.normal();
        uptodate = uptod.normal();
        couldnotget = couldnot.normal();
    }
    match informer {
        Ok(Some(version)) => println!(
            "{} on https://crates.io/crates/{}: {}",
            available, name, version
        ),
        Ok(None) => {
            println!("{uptodate} You already have the latest version.")
        }
        Err(ref e) => println!("{couldnotget} Error reported: {e}."),
    };
}

/// Asks the public for help
pub fn contribute() {
    println!();
    println!(
        "This project is currently only a vision. The Python package {} exists. ",
        get_prog_without_ext()
    );
    println!("The vision is to have a compatible program in Rust. I cannot do it myself, ");
    println!("but I can coordinate and merge your pull requests. Have a look at the repo ");
    println!("{}. Please help! Please contribute ", get_pkg_repository());
    println!("code to make this vision a reality, and to one day have a functional ");
    println!("{} crate. Safe!", get_prog_without_ext());
}

/// If necessary reads homeserver name for login and puts it into the Args.
/// If already set via --homeserver option, then it does nothing.
fn get_homeserver(ap: &mut Args) {
    while ap.homeserver.is_none() {
        print!("Enter your Matrix homeserver (e.g. https://some.homeserver.org): ");
        if let Err(e) = io::stdout().flush() {
            warn!("Warning: Failed to flush stdout: {e}");
        }
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            error!("Error: Unable to read user input");
            continue; // Skip to the next iteration if reading input fails
        }
        let trimmed_input = input.trim();
        if trimmed_input.is_empty() {
            error!("Error: Empty homeserver name is not allowed!");
        } else if let Err(e) = Url::parse(trimmed_input) {
            error!(
                "Error: The syntax is incorrect. Homeserver must be a valid URL! \
                Start with 'http://' or 'https://'. Details: {e}"
            );
        } else {
            ap.homeserver = Some(Url::parse(trimmed_input).unwrap()); // Safe to unwrap since we validated it
            debug!("homeserver is {}", ap.homeserver.as_ref().unwrap());
        }
    }
}

/// If necessary reads user name for login and puts it into the Args.
/// If already set via --user-login option, then it does nothing.
fn get_user_login(ap: &mut Args) {
    while ap.user_login.is_none() {
        print!("Enter your full Matrix username (e.g. @john:some.homeserver.org): ");
        if let Err(e) = io::stdout().flush() {
            warn!("Warning: Failed to flush stdout: {e}");
        }
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            error!("Error: Unable to read user input");
            continue; // Skip to the next iteration if reading input fails
        }
        let trimmed_input = input.trim();
        if trimmed_input.is_empty() {
            error!("Error: Empty username is not allowed!");
        } else if !is_valid_username(trimmed_input) {
            error!("Error: Invalid username format!");
        } else {
            ap.user_login = Some(trimmed_input.to_string());
            debug!("user_login is {trimmed_input}");
        }
    }
}

// validation function for username format
fn is_valid_username(username: &str) -> bool {
    // Check if it starts with '@', contains ':', etc.
    username.starts_with('@') && username.contains(':')
}

/// If necessary reads password for login and puts it into the Args.
/// If already set via --password option, then it does nothing.
fn get_password(ap: &mut Args) {
    while ap.password.is_none() {
        print!("Enter Matrix password for this user: ");
        // Flush stdout to ensure the prompt is displayed
        if let Err(e) = io::stdout().flush() {
            warn!("Warning: Failed to flush stdout: {e}");
        }
        // Handle potential errors from read_password
        match read_password() {
            Ok(password) => {
                let trimmed_password = password.trim();
                if trimmed_password.is_empty() {
                    error!("Error: Empty password is not allowed!");
                } else {
                    ap.password = Some(password);
                    // Hide password from debug log files
                    debug!("password is {}", "******");
                }
            }
            Err(e) => {
                error!("Error reading password: {e}");
            }
        }
    }
}

/// If necessary reads room_default for login and puts it into the Args.
/// If already set via --room_default option, then it does nothing.
fn get_room_default(ap: &mut Args) {
    while ap.room_default.is_none() {
        print!(
            "Enter name of one of your Matrix rooms that you want to use as default room  \
            (e.g. !someRoomId:some.homeserver.org): "
        );
        if let Err(e) = io::stdout().flush() {
            warn!("Warning: Failed to flush stdout: {e}");
        }
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            error!("Error: Unable to read user input");
            continue; // Skip to the next iteration if reading input fails
        }
        let trimmed_input = input.trim();
        if trimmed_input.is_empty() {
            error!("Error: Empty name of default room is not allowed!");
        } else if !is_valid_room_name(trimmed_input) {
            error!("Error: Invalid room name format for '{trimmed_input}'! Room name must start with '!' and contain exactly one ':'.");
        } else {
            ap.room_default = Some(trimmed_input.to_string());
            debug!("room_default is '{trimmed_input}'");
        }
    }
}

// Validation function for room name format
fn is_valid_room_name(name: &str) -> bool {
    name.starts_with('!') && name.matches(':').count() == 1
}

/// A room is either specified with --room or the default from credentials file is used
/// On error return None.
fn set_rooms(ap: &mut Args, default_room: &str) {
    debug!("set_rooms()");
    if ap.room.is_empty() {
        ap.room.push(default_room.to_string()); // since --room is empty, use default room from credentials
    }
}

// /// Before get_rooms() is called the rooms should have been updated with set_rooms() first.
// /// Get the user specified rooms (which might either have been specified with --room or
// /// be the default room from the credentials file).
// /// On error return None.
// fn get_rooms(ap: &Args) -> &Vec<String> {
//     debug!("get_rooms()");
//     &ap.room
// }

/// Get the default room id from the credentials file.
/// On error return None.
async fn get_room_default_from_credentials(client: &Client, profile: &ProfileConfig) -> String {
    let mut room = profile.default_room.clone();
    let hostname = profile
        .homeserver
        .as_ref()
        .and_then(|u| u.host_str())
        .unwrap_or("");
    convert_to_full_room_id(client, &mut room, hostname).await;
    room
}

/// A user is either specified with --user or the default from credentials file is used
/// On error return None.
fn set_users(ap: &mut Args, profile: &ProfileConfig) {
    debug!("set_users()");
    if ap.user.is_empty() {
        let duser = get_user_default_from_credentials(profile);
        ap.user.push(duser.to_string()); // since --user is empty, use default user from credentials
    }
}

/// Before get_users() is called the users should have been updated with set_users() first.
/// Get the user specified users (which might either have been specified with --user or
/// be the default user from the credentials file).
/// On error return None.
#[allow(dead_code)]
fn get_users(ap: &Args) -> &Vec<String> {
    debug!("get_users()");
    &ap.user
}

/// Get the default user id from the credentials file.
/// On error return None.
fn get_user_default_from_credentials(profile: &ProfileConfig) -> OwnedUserId {
    profile.user_id.clone()
}

/// Convert a vector of aliases that can contain short alias forms into
/// a vector of fully canonical aliases.
/// john and #john will be converted to #john:matrix.server.org.
/// vecstr: the vector of aliases
/// default_host: the default hostname like "matrix.server.org"
fn convert_to_full_room_aliases(vecstr: &mut Vec<String>, default_host: &str) {
    vecstr.retain(|x| !x.trim().is_empty());
    for el in vecstr {
        el.retain(|c| !c.is_whitespace());
        if el.starts_with('!') {
            warn!("A room id was given as alias. {:?}", el);
            continue;
        }
        if !el.starts_with('#') {
            el.insert(0, '#');
        }
        if !el.contains(':') {
            el.push(':');
            el.push_str(default_host);
        }
    }
}

// Replace shortcut "-" with room id of default room
fn replace_minus_with_default_room(vecstr: &mut Vec<String>, default_room: &str) {
    // There is no way to distringuish --get-room-info not being in CLI
    // and --get-room-info being in API without a room.
    // Hence it is not possible to say "if vector is empty let's use the default room".
    // The user has to specify something, we used "-".
    if vecstr.iter().any(|x| x.trim() == "-") {
        vecstr.push(default_room.to_string());
    }
    vecstr.retain(|x| x.trim() != "-");
}

/// Handle the --bootstrap CLI argument
pub(crate) async fn cli_bootstrap(client: &Client, ap: &mut Args) -> Result<(), Error> {
    info!("Bootstrap chosen.");
    crate::bootstrap(client, ap).await
}

/// Handle the --verify CLI argument
pub(crate) async fn cli_verify(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Verify chosen.");
    if ap.verify.is_none() {
        return Err(Error::UnsupportedCliParameter(
            "Argument --verify cannot be empty",
        ));
    }
    if !ap.verify.is_manual_device()
        && !ap.verify.is_manual_user()
        && !ap.verify.is_emoji()
        && !ap.verify.is_emoji_req()
    {
        error!(
            "Verify option '{:?}' currently not supported. \
            Use '{:?}', '{:?}', '{:?}' or {:?}' for the time being.",
            ap.verify,
            Verify::ManualDevice,
            Verify::ManualUser,
            Verify::Emoji,
            Verify::EmojiReq
        );
        return Err(Error::UnsupportedCliParameter(
            "Used --verify option is currently not supported",
        ));
    }
    crate::verify(client, ap).await
}

fn trim_newline(s: &mut String) -> &mut String {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
    s
}

/// Handle the --message CLI argument
pub(crate) async fn cli_message(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Message chosen.");
    if ap.message.is_empty() {
        return Ok(()); // nothing to do
    }
    let mut fmsgs: Vec<String> = Vec::new(); // formatted msgs
    for msg in ap.message.iter() {
        if msg.is_empty() {
            info!("Skipping empty text message.");
            continue;
        };
        if msg == "--" {
            info!("Skipping '--' text message as these are used to separate arguments.");
            continue;
        };
        // - map to - (stdin pipe)
        // \- maps to text r'-', a 1-letter message
        let fmsg = if msg == r"-" {
            let mut line = String::new();
            if stdin().is_terminal() {
                print!("Message: ");
                io::stdout().flush()?;
                io::stdin().read_line(&mut line)?;
            } else {
                io::stdin().read_to_string(&mut line)?;
            }
            // line.trim_end().to_string() // remove /n at end of string
            line.strip_suffix("\r\n")
                .or(line.strip_suffix("\n"))
                .unwrap_or(&line)
                .to_string() // remove /n at end of string
        } else if msg == r"_" {
            let mut eof = false;
            while !eof {
                let mut line = String::new();
                match io::stdin().read_line(&mut line) {
                    // If this function returns Ok(0), the stream has reached EOF.
                    Ok(n) => {
                        if n == 0 {
                            eof = true;
                            debug!("Reached EOF of pipe stream.");
                        } else {
                            debug!(
                                "Read {n} bytes containing \"{}\\n\" from pipe stream.",
                                trim_newline(&mut line.clone())
                            );
                            match message(
                                client,
                                &[line],
                                &ap.room,
                                &MessageOptions {
                                    code: ap.code,
                                    markdown: ap.markdown,
                                    notice: ap.notice,
                                    emote: ap.emote,
                                    html: ap.html,
                                },
                            )
                            .await
                            {
                                Ok(()) => {
                                    debug!("message from pipe stream sent successfully");
                                }
                                Err(ref e) => {
                                    error!(
                                        "Error: sending message from pipe stream reported {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                    Err(ref e) => {
                        error!("Error: reading from pipe stream reported {}", e);
                    }
                }
            }
            "".to_owned()
        } else if msg == r"\-" {
            "-".to_string()
        } else if msg == r"\_" {
            "_".to_string()
        } else if msg == r"\-\-" {
            "--".to_string()
        } else if msg == r"\-\-\-" {
            "---".to_string()
        } else {
            msg.to_string()
        };
        if !fmsg.is_empty() {
            fmsgs.push(fmsg);
        }
    }
    if fmsgs.is_empty() {
        return Ok(()); // nothing to do
    }
    message(
        client,
        &fmsgs,
        &ap.room,
        &MessageOptions {
            code: ap.code,
            markdown: ap.markdown,
            notice: ap.notice,
            emote: ap.emote,
            html: ap.html,
        },
    )
    .await // returning
}

/// Handle the --file CLI argument
pub(crate) async fn cli_file(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("File chosen.");
    if ap.file.is_empty() {
        return Ok(()); // nothing to do
    }
    let mut files: Vec<PathBuf> = Vec::new();
    for filename in &ap.file {
        match filename.to_str() {
            Some("") => info!("Skipping empty file name."),
            Some(r"-") => files.push(PathBuf::from("-")),
            Some(r"\-") => files.push(PathBuf::from(r"\-")),
            Some(_) => files.push(filename.clone()),
            None => {
                warn!("Skipping file with invalid UTF-8 path: {:?}", filename);
                continue;
            }
        }
    }
    // pb: label to attach to a stdin pipe data in case there is data piped in from stdin
    let pb: PathBuf = if !ap.file_name.is_empty() {
        ap.file_name[0].clone()
    } else {
        PathBuf::from("file")
    };
    file(
        client, &files, &ap.room, None, // label, use default filename
        None, // mime, guess it
        &pb,  // label for stdin pipe
    )
    .await // returning
}

/// Handle the --media-upload CLI argument
pub(crate) async fn cli_media_upload(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Media upload chosen.");
    media_upload(client, &ap.media_upload, &ap.mime, ap.output).await // returning
}

/// Handle the --media-download once CLI argument
pub(crate) async fn cli_media_download(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Media download chosen.");
    media_download(client, &ap.media_download, &ap.file_name, ap.output).await // returning
}

/// Handle the --media-delete once CLI argument
pub(crate) async fn cli_media_delete(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Media delete chosen.");
    media_delete(client, &ap.media_delete, ap.output).await // returning
}

/// Handle the --media-mxc-to-http once CLI argument
pub(crate) async fn cli_media_mxc_to_http(ap: &Args, homeserver: &Url) -> Result<(), Error> {
    info!("Media mxc_to_http chosen.");
    media_mxc_to_http(&ap.media_mxc_to_http, homeserver, ap.output).await // returning
}

/// Handle the --listen once CLI argument
pub(crate) async fn cli_listen_once(
    client: &Client,
    ap: &Args,
    profile: &ProfileConfig,
) -> Result<(), Error> {
    info!("Listen Once chosen.");
    listen_once(client, ap.listen_self, crate::whoami(profile), ap.output).await
    // returning
}

/// Handle the --listen forever CLI argument
pub(crate) async fn cli_listen_forever(
    client: &Client,
    ap: &Args,
    profile: &ProfileConfig,
) -> Result<(), Error> {
    info!("Listen Forever chosen.");
    listen_forever(client, ap.listen_self, crate::whoami(profile), ap.output).await
    // returning
}

/// Handle the --listen tail CLI argument
pub(crate) async fn cli_listen_tail(
    client: &Client,
    ap: &Args,
    profile: &ProfileConfig,
) -> Result<(), Error> {
    info!("Listen Tail chosen.");
    listen_tail(
        client,
        &ap.room,
        ap.tail,
        ap.listen_self,
        crate::whoami(profile),
        ap.output,
    )
    .await // returning
}

/// Handle the --listen all CLI argument
pub(crate) async fn cli_listen_all(
    client: &Client,
    ap: &Args,
    profile: &ProfileConfig,
) -> Result<(), Error> {
    info!("Listen All chosen.");
    listen_all(
        client,
        &ap.room,
        ap.listen_self,
        crate::whoami(profile),
        ap.output,
    )
    .await // returning
}

/// Handle the --devices CLI argument
pub(crate) async fn cli_devices(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Devices chosen.");
    crate::devices(client, ap.output).await // returning
}

/// Utility function, returns user_id of itself
pub(crate) fn whoami(profile: &ProfileConfig) -> OwnedUserId {
    profile.user_id.clone()
}

/// Handle the --whoami CLI argument
pub(crate) fn cli_whoami(ap: &Args, profile: &ProfileConfig) -> Result<(), Error> {
    info!("Whoami chosen.");
    let whoami = crate::whoami(profile);
    match ap.output {
        Output::Text => println!("{}", whoami),
        Output::JsonSpec => (),
        Output::Json | Output::JsonMax => {
            let info = json!({
                "user_id" : whoami,
            });
            println!("{}", serde_json::to_string(&info)?);
        }
    }
    Ok(())
}

/// Handle the --get-room-info CLI argument
pub(crate) async fn cli_get_room_info(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Get-room-info chosen.");
    // note that get_room_info vector is NOT empty.
    // If it were empty this function would not be called.
    crate::get_room_info(client, &ap.get_room_info, ap.output).await
}

/// Handle the --rooms CLI argument
pub(crate) async fn cli_rooms(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Rooms chosen.");
    crate::rooms(client, ap.output).await
}

/// Handle the --invited-rooms CLI argument
pub(crate) async fn cli_invited_rooms(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Invited-rooms chosen.");
    crate::invited_rooms(client, ap.output).await
}

/// Handle the --joined-rooms CLI argument
pub(crate) async fn cli_joined_rooms(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Joined-rooms chosen.");
    crate::joined_rooms(client, ap.output).await
}

/// Handle the --left-rooms CLI argument
pub(crate) async fn cli_left_rooms(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Left-rooms chosen.");
    crate::left_rooms(client, ap.output).await
}

/// Handle the --room-create CLI argument
pub(crate) async fn cli_room_create(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-create chosen.");
    crate::room_create(
        client,
        false,
        match &ap.visibility {
            Visibility::Private => !ap.plain.unwrap_or(false), // private rooms are encrypted by default
            Visibility::Public => !ap.plain.unwrap_or(true),   // public rooms are plain by default
            _ => !ap.plain.unwrap_or(false),
        },
        &[],
        &ap.room_create,
        &ap.name,
        &ap.topic,
        ap.output,
        ap.visibility.clone(),
    )
    .await
}

/// Handle the --room-create CLI argument
pub(crate) async fn cli_room_dm_create(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-dm-create chosen.");
    crate::room_create(
        client,
        true,
        match &ap.visibility {
            Visibility::Private => !ap.plain.unwrap_or(false), // private rooms are encrypted by default
            Visibility::Public => !ap.plain.unwrap_or(true),   // public rooms are plain by default
            _ => !ap.plain.unwrap_or(false),
        },
        &ap.room_dm_create,
        &ap.alias,
        &ap.name,
        &ap.topic,
        ap.output,
        ap.visibility.clone(),
    )
    .await
}

/// Handle the --room-leave CLI argument
pub(crate) async fn cli_room_leave(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-leave chosen.");
    crate::room_leave(client, &ap.room_leave, ap.output).await
}

/// Handle the --room-forget CLI argument
pub(crate) async fn cli_room_forget(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-forget chosen.");
    crate::room_forget(client, &ap.room_forget, ap.output).await
}

/// Handle the --room-invite CLI argument
pub(crate) async fn cli_room_invite(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-invite chosen.");
    crate::room_invite(client, &ap.room_invite, &ap.user, ap.output).await
}

/// Handle the --room-join CLI argument
pub(crate) async fn cli_room_join(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-join chosen.");
    crate::room_join(client, &ap.room_join, ap.output).await
}

/// Handle the --room-ban CLI argument
pub(crate) async fn cli_room_ban(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-ban chosen.");
    crate::room_ban(client, &ap.room_ban, &ap.user, ap.output).await
}

/// Handle the --room-unban CLI argument
pub(crate) async fn cli_room_unban(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-unban chosen.");
    crate::room_unban(client, &ap.room_unban, &ap.user, ap.output).await
}

/// Handle the --room-kick CLI argument
pub(crate) async fn cli_room_kick(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-kick chosen.");
    crate::room_kick(client, &ap.room_kick, &ap.user, ap.output).await
}

/// Handle the --room-resolve_alias CLI argument
pub(crate) async fn cli_room_resolve_alias(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-resolve-alias chosen.");
    crate::room_resolve_alias(client, &ap.room_resolve_alias, ap.output).await
}

/// Handle the --room-enable-encryption CLI argument
pub(crate) async fn cli_room_enable_encryption(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-enable-encryption chosen.");
    crate::room_enable_encryption(client, &ap.room_enable_encryption, ap.output).await
}

/// Handle the --get-avatar CLI argument
pub(crate) async fn cli_get_avatar(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Get-avatar chosen.");
    if let Some(path) = ap.get_avatar.as_ref() {
        crate::get_avatar(client, path, ap.output).await
    } else {
        Err(Error::MissingCliParameter)
    }
}

/// Handle the --set-avatar CLI argument
pub(crate) async fn cli_set_avatar(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Set-avatar chosen.");
    if let Some(path) = ap.set_avatar.as_ref() {
        crate::set_avatar(client, path, ap.output).await
    } else {
        Err(Error::MissingCliParameter)
    }
}

/// Handle the --get-avatar-url CLI argument
pub(crate) async fn cli_get_avatar_url(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Get-avatar-url chosen.");
    crate::get_avatar_url(client, ap.output).await
}

/// Handle the --set-avatar_url CLI argument
pub(crate) async fn cli_set_avatar_url(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Set-avatar-url chosen.");
    if let Some(mxc_uri) = ap.set_avatar_url.as_ref() {
        crate::set_avatar_url(client, mxc_uri, ap.output).await
    } else {
        Err(Error::MissingCliParameter)
    }
}

/// Handle the --unset-avatar_url CLI argument
pub(crate) async fn cli_unset_avatar_url(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Unset-avatar-url chosen.");
    crate::unset_avatar_url(client, ap.output).await
}

/// Handle the --get-display-name CLI argument
pub(crate) async fn cli_get_display_name(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Get-display-name chosen.");
    crate::get_display_name(client, ap.output).await
}

/// Handle the --set-display-name CLI argument
pub(crate) async fn cli_set_display_name(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Set-display-name chosen.");
    if let Some(name) = ap.set_display_name.as_ref() {
        crate::set_display_name(client, name, ap.output).await
    } else {
        Err(Error::MissingCliParameter)
    }
}

/// Handle the --get-profile CLI argument
pub(crate) async fn cli_get_profile(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Get-profile chosen.");
    crate::get_profile(client, ap.output).await
}

/// Handle the --get-masterkey CLI argument
pub(crate) async fn cli_get_masterkey(
    client: &Client,
    ap: &Args,
    profile: &ProfileConfig,
) -> Result<(), Error> {
    info!("Get-masterkey chosen.");
    crate::get_masterkey(client, profile.user_id.clone(), ap.output).await
}

/// Handle the --room-get-visibility CLI argument
pub(crate) async fn cli_room_get_visibility(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-get-visibility chosen.");
    crate::room_get_visibility(client, &ap.room_get_visibility, ap.output).await
}

/// Handle the --room-get-state CLI argument
pub(crate) async fn cli_room_get_state(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Room-get-state chosen.");
    crate::room_get_state(client, &ap.room_get_state, ap.output).await
}

/// Handle the --joined-members CLI argument
pub(crate) async fn cli_joined_members(client: &Client, ap: &Args) -> Result<(), Error> {
    info!("Joined-members chosen.");
    crate::joined_members(client, &ap.joined_members, ap.output).await
}

/// Handle the --delete-device CLI argument
pub(crate) async fn cli_delete_device(client: &Client, ap: &mut Args) -> Result<(), Error> {
    info!("Delete-device chosen.");
    crate::delete_devices_pre(client, ap).await
}

/// Handle the --logout CLI argument
pub(crate) async fn cli_logout(
    client: &Client,
    ap: &mut Args,
    session_json: SessionJson,
    sqlite_store: SqliteStore,
) -> Result<(), Error> {
    info!("Logout chosen.");
    match ap.logout {
        Logout::None => Ok(()),
        Logout::Me => crate::logout(client, ap, session_json, sqlite_store).await,
        Logout::All => {
            ap.delete_device = vec!["*".to_owned()];
            match cli_delete_device(client, ap).await {
                Ok(_) => {
                    info!("Logout caused all devices to be deleted.");
                    Ok(())
                }
                Err(e) => {
                    error!(
                        "Error: Failed to delete all devices, but we remove local device id anyway. {:?}",
                        e
                    );
                    Err(e)
                }
            }
        }
    }
}

const DEFAULT_ENCRYPTION_SETTINGS: EncryptionSettings = EncryptionSettings {
    auto_enable_cross_signing: true,
    auto_enable_backups: true,
    backup_download_strategy: BackupDownloadStrategy::AfterDecryptionFailure,
};

/// Build a [`matrix_sdk::Client`] pointed at the given homeserver URL, using
/// the profile's SQLite store and the specified timeout.
async fn build_matrix_client(
    homeserver: &Url,
    settings: &Settings,
    timeout: u64,
) -> Result<Client, Error> {
    use matrix_sdk::config::RequestConfig;
    use std::time::Duration;

    let req_timeout = Duration::from_secs(timeout);
    let req_config = RequestConfig::new()
        .timeout(req_timeout)
        .max_retry_time(req_timeout);

    Client::builder()
        .homeserver_url(homeserver)
        .request_config(req_config)
        .with_encryption_settings(DEFAULT_ENCRYPTION_SETTINGS)
        .sqlite_store(&settings.sqlite_dir, None)
        .build()
        .await
        .map_err(|e| {
            error!("Cannot build Matrix client: {e}");
            Error::LoginFailed
        })
}

/// We need your code contributions! Please add features and make PRs! :pray: :clap:
#[tokio::main]
async fn main() -> Result<(), Error> {
    if std::env::args().nth(1).is_none() {
        eprintln!("Missing arguments, please see --help");
        process::exit(1);
    }
    let mut ap = Args::parse();
    let mut errcount = 0;
    let mut result: Result<(), Error> = Ok(());

    // handle log level and debug options
    let env_org_rust_log = env::var("RUST_LOG").unwrap_or_default().to_uppercase();
    // println!("Original log_level option is {:?}", ap.log_level);
    // println!("Original RUST_LOG is {:?}", &env_org_rust_log);
    match ap.debug.cmp(&1) {
        Ordering::Equal => {
            // -d overwrites --log-level
            let llvec = vec![LogLevel::Debug];
            ap.log_level = Some(llvec);
        }
        Ordering::Greater => {
            // -d overwrites --log-level
            let mut llvec = vec![LogLevel::Debug];
            llvec.push(LogLevel::Debug);
            ap.log_level = Some(llvec);
        }
        Ordering::Less => (),
    }
    match ap.log_level.clone() {
        None => {
            tracing_subscriber::fmt()
                .with_writer(io::stderr)
                .with_env_filter(EnvFilter::from_default_env()) // support the standard RUST_LOG env variable
                .init();
            debug!("Neither --debug nor --log-level was used. Using environment vaiable RUST_LOG.");
        }
        Some(llvec) => {
            if llvec.len() == 1 {
                if llvec[0].is_none() {
                    return Err(Error::UnsupportedCliParameter(
                        "Value 'none' not allowed for --log-level argument",
                    ));
                }
                // .with_env_filter("matrix_commander_rs=debug") // only set matrix_commander_rs
                let mut rlogstr: String = BIN_NAME_UNDERSCORE.to_owned();
                rlogstr.push('='); // add char
                rlogstr.push_str(&llvec[0].to_string());
                tracing_subscriber::fmt()
                    .with_writer(io::stderr)
                    .with_env_filter(rlogstr.clone()) // support the standard RUST_LOG env variable for default value
                    .init();
                debug!(
                    "The --debug or --log-level was used once or with one value. \
                    Specifying logging equivalent to RUST_LOG seting of '{}'.",
                    rlogstr
                );
            } else {
                if llvec[0].is_none() || llvec[1].is_none() {
                    return Err(Error::UnsupportedCliParameter(
                        "Value 'none' not allowed for --log-level argument",
                    ));
                }
                // RUST_LOG="error,matrix_commander_rs=debug"  .. This will only show matrix-comander-rs
                // debug info, and erors for all other modules
                let mut rlogstr: String = llvec[1].to_string().to_owned();
                rlogstr.push(','); // add char
                rlogstr.push_str(BIN_NAME_UNDERSCORE);
                rlogstr.push('=');
                rlogstr.push_str(&llvec[0].to_string());
                tracing_subscriber::fmt()
                    .with_writer(io::stderr)
                    .with_env_filter(rlogstr.clone())
                    .init();
                debug!(
                    "The --debug or --log-level was used twice or with two values. \
                    Specifying logging equivalent to RUST_LOG seting of '{}'.",
                    rlogstr
                );
            }
            if llvec.len() > 2 {
                debug!("The --log-level option was incorrectly used more than twice. Ignoring third and further use.")
            }
        }
    }
    if ap.debug > 0 {
        info!("The --debug option overwrote the --log-level option.")
    }
    if ap.debug > 2 {
        debug!("The --debug option was incorrectly used more than twice. Ignoring third and further use.")
    }
    debug!("Original RUST_LOG env var is '{}'", env_org_rust_log);
    debug!(
        "Final RUST_LOG env var is '{}'",
        env::var("RUST_LOG").unwrap_or_default().to_uppercase()
    );
    debug!("Final log-level option is {:?}", ap.log_level);
    if enabled!(Level::TRACE) {
        debug!(
            "Log level of module {} is set to TRACE.",
            get_prog_without_ext()
        );
    } else if enabled!(Level::DEBUG) {
        debug!(
            "Log level of module {} is set to DEBUG.",
            get_prog_without_ext()
        );
    }
    debug!("Version is {}", get_version());
    debug!("Package name is {}", get_pkg_name());
    debug!("Repo is {}", get_pkg_repository());
    debug!("contribute flag is disabled (removed from CLI)");
    debug!("version option is set to {:?}", ap.version);
    debug!("debug flag is {}", ap.debug);
    debug!("log-level option is {:?}", ap.log_level);
    debug!("verbose option is {}", ap.verbose);
    debug!("plain flag is {:?}", ap.plain);
    debug!("profile option is {:?}", ap.profile);
    debug!("store option is {:?}", ap.store);
    debug!("login option is {:?}", ap.login);
    debug!("bootstrap flag is {:?}", ap.bootstrap);
    debug!("verify flag is {:?}", ap.verify);
    debug!("message option is {:?}", ap.message);
    debug!("logout option is {:?}", ap.logout);
    debug!("homeserver option is {:?}", ap.homeserver);
    debug!("user-login option is {:?}", ap.user_login);
    debug!("password option is {:?}", ap.password);
    debug!("device option is {:?}", ap.device);
    debug!("room-default option is {:?}", ap.room_default);
    debug!("devices flag is {:?}", ap.devices);
    debug!("timeout option is {:?}", ap.timeout);
    debug!("markdown flag is {:?}", ap.markdown);
    debug!("code flag is {:?}", ap.code);
    debug!("room option is {:?}", ap.room);
    debug!("file option is {:?}", ap.file);
    debug!("notice flag is {:?}", ap.notice);
    debug!("emote flag is {:?}", ap.emote);
    debug!("sync option is {:?}", ap.sync);
    debug!("listen option is {:?}", ap.listen);
    debug!("tail option is {:?}", ap.tail);
    debug!("listen-self flag is {:?}", ap.listen_self);
    debug!("whoami flag is {:?}", ap.whoami);
    debug!("output option is {:?}", ap.output);
    debug!("get-room-info option is {:?}", ap.get_room_info);
    debug!("file-name option is {:?}", ap.file_name);
    debug!("room-create option is {:?}", ap.room_create);
    debug!("room-dm-create option is {:?}", ap.room_dm_create);
    debug!("room-leave option is {:?}", ap.room_leave);
    debug!("room-forget option is {:?}", ap.room_forget);
    debug!("room-invite option is {:?}", ap.room_invite);
    debug!("room-join option is {:?}", ap.room_join);
    debug!("room-ban option is {:?}", ap.room_ban);
    debug!("room-unban option is {:?}", ap.room_unban);
    debug!("room-kick option is {:?}", ap.room_kick);
    debug!("room-resolve-alias option is {:?}", ap.room_resolve_alias);
    debug!(
        "room-enable-encryption option is {:?}",
        ap.room_enable_encryption
    );
    debug!("alias option is {:?}", ap.alias);
    debug!("name option is {:?}", ap.name);
    debug!("topic-create option is {:?}", ap.topic);
    debug!("rooms option is {:?}", ap.rooms);
    debug!("invited-rooms option is {:?}", ap.invited_rooms);
    debug!("joined-rooms option is {:?}", ap.joined_rooms);
    debug!("left-rooms option is {:?}", ap.left_rooms);
    debug!("room-get-visibility option is {:?}", ap.room_get_visibility);
    debug!("room-get-state option is {:?}", ap.room_get_state);
    debug!("joined-members option is {:?}", ap.joined_members);
    debug!("delete-device option is {:?}", ap.delete_device);
    debug!("user option is {:?}", ap.user);
    debug!("get-avatar option is {:?}", ap.get_avatar);
    debug!("set-avatar option is {:?}", ap.set_avatar);
    debug!("get-avatar_url flag is {:?}", ap.get_avatar_url);
    debug!("set-avatar_url option is {:?}", ap.set_avatar_url);
    debug!("unset-avatar_url flag is {:?}", ap.unset_avatar_url);
    debug!("get-display-name option is {:?}", ap.get_display_name);
    debug!("set-display-name option is {:?}", ap.set_display_name);
    debug!("get-profile option is {:?}", ap.get_profile);
    debug!("media-upload option is {:?}", ap.media_upload);
    debug!("media-download option is {:?}", ap.media_download);
    debug!("media-delete option is {:?}", ap.media_delete);
    debug!("media-mxc-to-http option is {:?}", ap.media_mxc_to_http);
    debug!("mime option is {:?}", ap.mime);
    debug!("get-masterkey option is {:?}", ap.get_masterkey);

    match ap.version {
        None => (),                              // do nothing
        Some(None) => crate::version(ap.output), // print version
        Some(Some(Version::Check)) => crate::version_check(),
    }

    if ap.config {
        return Ok(());
    }

    if ap.usage {
        crate::usage();
        return Ok(());
    };
    if ap.help {
        crate::help();
        return Ok(());
    };
    if ap.manual {
        crate::manual();
        return Ok(());
    };
    if ap.readme {
        crate::readme().await;
        return Ok(());
    };

    // -m not used but data being piped into stdin?
    if ap.message.is_empty() && !stdin().is_terminal() {
        // make it more compatible with the Python version of this tool
        debug!(
            "-m is empty, but there is something piped into stdin. Let's assume '-m -' \
            and read and send the information piped in on stdin."
        );
        ap.message.push("-".to_string());
    };

    if !(!(ap.login == LoginCLI::None)
        // get actions
        || ap.whoami
        || ap.bootstrap
        || !ap.verify.is_none()
        || ap.devices
        || !ap.get_room_info.is_empty()
        || ap.rooms
        || ap.invited_rooms
        || ap.joined_rooms
        || ap.left_rooms
        || !ap.room_get_visibility.is_empty()
        || !ap.room_get_state.is_empty()
        || !ap.joined_members.is_empty()
        || !ap.room_resolve_alias.is_empty()
        || ap.get_avatar.is_some()
        || ap.get_avatar_url
        || ap.get_display_name
        || ap.get_profile
        || !ap.media_download.is_empty()
        || !ap.media_mxc_to_http.is_empty()
        || ap.get_masterkey
        // set actions
        || !ap.room_create.is_empty()
        || !ap.room_dm_create.is_empty()
        || !ap.room_leave.is_empty()
        || !ap.room_forget.is_empty()
        || !ap.room_invite.is_empty()
        || !ap.room_join.is_empty()
        || !ap.room_ban.is_empty()
        || !ap.room_unban.is_empty()
        || !ap.room_kick.is_empty()
        || !ap.delete_device.is_empty()
        || ap.set_avatar.is_some()
        || ap.set_avatar_url.is_some()
        || ap.unset_avatar_url
        || ap.set_display_name.is_some()
        || !ap.room_enable_encryption.is_empty()
        || !ap.media_upload.is_empty()
        || !ap.media_delete.is_empty()
        // send and listen actions
        || !ap.message.is_empty()
        || !ap.file.is_empty()
        || ap.listen.is_once()
        || ap.listen.is_forever()
        || ap.listen.is_tail()
        || ap.tail > 0
        || ap.listen.is_all()
        || !ap.logout.is_none())
    {
        debug!("There are no more actions to take. No need to connect to server. Quitting.");
        debug!("Good bye");
        return Ok(());
    }

    let (settings, client) = login::LoginFlow::login(&mut ap).await?;

    let homeserver = settings
        .profile
        .homeserver
        .clone()
        .ok_or(Error::HomeserverNotSet)?;

    // Place all the calls here that work without a server connection
    // whoami: works even without client (server connection)
    if ap.whoami {
        match crate::cli_whoami(&ap, &settings.profile) {
            Ok(ref _n) => debug!("crate::whoami successful"),
            Err(e) => {
                error!("Error: crate::whoami reported {}", e);
                errcount += 1;
                result = Err(e);
            }
        };
    };

    convert_to_full_mxc_uris(
        &mut ap.media_mxc_to_http,
        homeserver.host_str().unwrap_or(""),
    )
    .await; // convert short mxcs to full mxc uris

    // media_mxc_to_http works without client (server connection)
    if !ap.media_mxc_to_http.is_empty() {
        match crate::cli_media_mxc_to_http(&ap, &homeserver).await {
            Ok(ref _n) => debug!("crate::media_mxc_to_http successful"),
            Err(e) => {
                error!("Error: crate::media_mxc_to_http reported {}", e);
                errcount += 1;
                result = Err(e);
            }
        };
    };

    {
        let client = &client;
        debug!("A valid client connection has been established with server.");

        // Sync with the server so the local SQLite store is populated with current
        // room state. Without this, client.rooms() and similar calls return empty
        // results even when the user is a member of rooms.
        // Must run before preprocessing because replace_star_with_rooms()
        // also calls client.rooms() to expand '*' into actual room IDs.
        // listen_* functions do their own internal sync, so the only downside
        // for --listen use-cases is one extra round-trip (avoidable via --sync off).
        match sync_once(client, ap.timeout, ap.sync).await {
            Ok(()) => debug!("sync_once successful"),
            Err(ref e) => warn!("sync_once reported error {:?}. Continuing.", e),
        };

        // pre-processing of CLI arguments, filtering, replacing shortcuts, etc.
        let default_room = get_room_default_from_credentials(client, &settings.profile).await;
        // Todo: port number is not handled in hostname, could be matrix.server.org:90
        let hostname = homeserver.host_str().unwrap_or(""); // matrix.server.org
        set_rooms(&mut ap, &default_room); // if no rooms in --room, set rooms to default room from credentials file
        set_users(&mut ap, &settings.profile); // if no users in --user, set users to default user from credentials file

        replace_minus_with_default_room(&mut ap.room_leave, &default_room); // convert '-' to default room
        convert_to_full_room_ids(client, &mut ap.room_leave, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.room_forget, &default_room); // convert '-' to default room
        convert_to_full_room_ids(client, &mut ap.room_forget, hostname).await; // convert short ids, short aliases and aliases to full room ids

        convert_to_full_room_aliases(&mut ap.room_resolve_alias, hostname); // convert short aliases to full aliases

        replace_minus_with_default_room(&mut ap.room_enable_encryption, &default_room); // convert '-' to default room
        convert_to_full_room_ids(client, &mut ap.room_enable_encryption, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.get_room_info, &default_room); // convert '-' to default room
        convert_to_full_room_ids(client, &mut ap.get_room_info, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.room_invite, &default_room); // convert '-' to default room
        convert_to_full_room_ids(client, &mut ap.room_invite, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.room_join, &default_room); // convert '-' to default room
        convert_to_full_room_ids(client, &mut ap.room_join, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.room_ban, &default_room); // convert '-' to default room
        convert_to_full_room_ids(client, &mut ap.room_ban, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.room_unban, &default_room); // convert '-' to default room
        convert_to_full_room_ids(client, &mut ap.room_unban, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.room_kick, &default_room); // convert '-' to default room
        convert_to_full_room_ids(client, &mut ap.room_kick, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.room_get_visibility, &default_room); // convert '-' to default room
        replace_star_with_rooms(client, &mut ap.room_get_visibility); // convert '*' to full list of rooms
        convert_to_full_room_ids(client, &mut ap.room_get_visibility, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.room_get_state, &default_room); // convert '-' to default room
        replace_star_with_rooms(client, &mut ap.room_get_state); // convert '*' to full list of rooms
        convert_to_full_room_ids(client, &mut ap.room_get_state, hostname).await; // convert short ids, short aliases and aliases to full room ids

        replace_minus_with_default_room(&mut ap.joined_members, &default_room); // convert '-' to default room
        replace_star_with_rooms(client, &mut ap.joined_members); // convert '*' to full list of rooms
        convert_to_full_room_ids(client, &mut ap.joined_members, hostname).await; // convert short ids, short aliases and aliases to full room ids

        convert_to_full_user_ids(&mut ap.room_dm_create, hostname);
        ap.room_dm_create.retain(|x| !x.trim().is_empty());

        convert_to_full_alias_ids(&mut ap.alias, hostname);
        ap.alias.retain(|x| !x.trim().is_empty());

        convert_to_full_mxc_uris(&mut ap.media_download, hostname).await; // convert short mxcs to full mxc uris

        convert_to_full_mxc_uris(&mut ap.media_delete, hostname).await; // convert short mxcs to full mxc uris

        if ap.tail > 0 {
            // overwrite --listen if user has chosen both
            if !ap.listen.is_never() && !ap.listen.is_tail() {
                warn!(
                    "Two contradicting listening methods were specified. \
                    Overwritten with --tail. Will use '--listen tail'. {:?} {}",
                    ap.listen, ap.tail
                )
            }
            ap.listen = Listen::Tail
        }

        // top-priority actions
        if ap.bootstrap {
            match crate::cli_bootstrap(client, &mut ap).await {
                Ok(ref _n) => debug!("crate::bootstrap successful"),
                Err(e) => {
                    error!("Error: crate::bootstrap reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.verify.is_none() {
            match crate::cli_verify(client, &ap).await {
                Ok(ref _n) => debug!("crate::verify successful"),
                Err(e) => {
                    error!("Error: crate::verify reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        // get actions

        if ap.devices {
            match crate::cli_devices(client, &ap).await {
                Ok(ref _n) => debug!("crate::devices successful"),
                Err(e) => {
                    error!("Error: crate::devices reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.get_room_info.is_empty() {
            match crate::cli_get_room_info(client, &ap).await {
                Ok(ref _n) => debug!("crate::get_room_info successful"),
                Err(e) => {
                    error!("Error: crate::get_room_info reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.rooms {
            match crate::cli_rooms(client, &ap).await {
                Ok(ref _n) => debug!("crate::rooms successful"),
                Err(e) => {
                    error!("Error: crate::rooms reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.invited_rooms {
            match crate::cli_invited_rooms(client, &ap).await {
                Ok(ref _n) => debug!("crate::invited_rooms successful"),
                Err(e) => {
                    error!("Error: crate::invited_rooms reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.joined_rooms {
            match crate::cli_joined_rooms(client, &ap).await {
                Ok(ref _n) => debug!("crate::joined_rooms successful"),
                Err(e) => {
                    error!("Error: crate::joined_rooms reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.left_rooms {
            match crate::cli_left_rooms(client, &ap).await {
                Ok(ref _n) => debug!("crate::left_rooms successful"),
                Err(e) => {
                    error!("Error: crate::left_rooms reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_get_visibility.is_empty() {
            match crate::cli_room_get_visibility(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_get_visibility successful"),
                Err(e) => {
                    error!("Error: crate::room_get_visibility reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_get_state.is_empty() {
            match crate::cli_room_get_state(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_get_state successful"),
                Err(e) => {
                    error!("Error: crate::room_get_state reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.joined_members.is_empty() {
            match crate::cli_joined_members(client, &ap).await {
                Ok(ref _n) => debug!("crate::joined_members successful"),
                Err(e) => {
                    error!("Error: crate::joined_members reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_resolve_alias.is_empty() {
            match crate::cli_room_resolve_alias(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_resolve_alias successful"),
                Err(e) => {
                    error!("Error: crate::room_resolve_alias reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.get_avatar.is_some() {
            match crate::cli_get_avatar(client, &ap).await {
                Ok(ref _n) => debug!("crate::get_avatar successful"),
                Err(e) => {
                    error!("Error: crate::get_avatar reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.get_avatar_url {
            match crate::cli_get_avatar_url(client, &ap).await {
                Ok(ref _n) => debug!("crate::get_avatar_url successful"),
                Err(e) => {
                    error!("Error: crate::get_avatar_url reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.get_display_name {
            match crate::cli_get_display_name(client, &ap).await {
                Ok(ref _n) => debug!("crate::get_display_name successful"),
                Err(e) => {
                    error!("Error: crate::get_display_name reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.get_profile {
            match crate::cli_get_profile(client, &ap).await {
                Ok(ref _n) => debug!("crate::get_profile successful"),
                Err(e) => {
                    error!("Error: crate::get_profile reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.get_masterkey {
            match crate::cli_get_masterkey(client, &ap, &settings.profile).await {
                Ok(ref _n) => debug!("crate::get_masterkey successful"),
                Err(e) => {
                    error!("Error: crate::get_masterkey reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.media_download.is_empty() {
            match crate::cli_media_download(client, &ap).await {
                Ok(ref _n) => debug!("crate::media_download successful"),
                Err(e) => {
                    error!("Error: crate::media_download reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        // set actions

        if !ap.room_create.is_empty() {
            match crate::cli_room_create(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_create successful"),
                Err(e) => {
                    error!("Error: crate::room_create reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_dm_create.is_empty() {
            match crate::cli_room_dm_create(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_dm_create successful"),
                Err(e) => {
                    error!("Error: crate::room_dm_create reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_leave.is_empty() {
            error!(
                "There is a bug in the matrix-sdk library and hence this is not working \
                properly at the moment. It will start working once matrix-sdk v0.7 is released."
            );
            match crate::cli_room_leave(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_leave successful"),
                Err(e) => {
                    error!("Error: crate::room_leave reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_forget.is_empty() {
            error!(
                "There is a bug in the matrix-sdk library and hence this is not working \
                properly at the moment. It might start working once matrix-sdk v0.7 is released."
            );
            match crate::cli_room_forget(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_forget successful"),
                Err(e) => {
                    error!("Error: crate::room_forget reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_invite.is_empty() {
            match crate::cli_room_invite(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_invite successful"),
                Err(e) => {
                    error!("Error: crate::room_invite reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_join.is_empty() {
            match crate::cli_room_join(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_join successful"),
                Err(e) => {
                    error!("Error: crate::room_join reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_ban.is_empty() {
            match crate::cli_room_ban(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_ban successful"),
                Err(e) => {
                    error!("Error: crate::room_ban reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_unban.is_empty() {
            match crate::cli_room_unban(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_unban successful"),
                Err(e) => {
                    error!("Error: crate::room_unban reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_kick.is_empty() {
            match crate::cli_room_kick(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_kick successful"),
                Err(e) => {
                    error!("Error: crate::room_kick reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.delete_device.is_empty() {
            match crate::cli_delete_device(client, &mut ap).await {
                Ok(ref _n) => debug!("crate::delete_device successful"),
                Err(e) => {
                    error!("Error: crate::delete_device reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.set_avatar.is_some() {
            match crate::cli_set_avatar(client, &ap).await {
                Ok(ref _n) => debug!("crate::set_avatar successful"),
                Err(e) => {
                    error!("Error: crate::set_avatar reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.set_avatar_url.is_some() {
            match crate::cli_set_avatar_url(client, &ap).await {
                Ok(ref _n) => debug!("crate::set_avatar_url successful"),
                Err(e) => {
                    error!("Error: crate::set_avatar_url reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.unset_avatar_url {
            match crate::cli_unset_avatar_url(client, &ap).await {
                Ok(ref _n) => debug!("crate::set_avatar_url successful"),
                Err(e) => {
                    error!("Error: crate::set_avatar_url reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if ap.set_display_name.is_some() {
            match crate::cli_set_display_name(client, &ap).await {
                Ok(ref _n) => debug!("crate::set_display_name successful"),
                Err(e) => {
                    error!("Error: crate::set_display_name reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.room_enable_encryption.is_empty() {
            match crate::cli_room_enable_encryption(client, &ap).await {
                Ok(ref _n) => debug!("crate::room_enable_encryption successful"),
                Err(e) => {
                    error!("Error: crate::room_enable_encryption reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.media_upload.is_empty() {
            match crate::cli_media_upload(client, &ap).await {
                Ok(ref _n) => debug!("crate::media_upload successful"),
                Err(e) => {
                    error!("Error: crate::media_upload reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        if !ap.media_delete.is_empty() {
            match crate::cli_media_delete(client, &ap).await {
                Ok(ref _n) => debug!("crate::media_delete successful"),
                Err(e) => {
                    error!("Error: crate::media_delete reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        // send text message(s)
        if !ap.message.is_empty() {
            match crate::cli_message(client, &ap).await {
                Ok(ref _n) => debug!("crate::message successful"),
                Err(e) => {
                    error!("Error: crate::message reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        // send file(s)
        if !ap.file.is_empty() {
            match crate::cli_file(client, &ap).await {
                Ok(ref _n) => debug!("crate::file successful"),
                Err(e) => {
                    error!("Error: crate::file reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        // listen once
        if ap.listen.is_once() {
            match crate::cli_listen_once(client, &ap, &settings.profile).await {
                Ok(ref _n) => debug!("crate::listen_once successful"),
                Err(e) => {
                    error!("Error: crate::listen_once reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        // listen forever
        if ap.listen.is_forever() {
            match crate::cli_listen_forever(client, &ap, &settings.profile).await {
                Ok(ref _n) => debug!("crate::listen_forever successful"),
                Err(e) => {
                    error!("Error: crate::listen_forever reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        // listen tail
        if ap.listen.is_tail() {
            match crate::cli_listen_tail(client, &ap, &settings.profile).await {
                Ok(ref _n) => debug!("crate::listen_tail successful"),
                Err(e) => {
                    error!("Error: crate::listen_tail reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        // listen all
        if ap.listen.is_all() {
            match crate::cli_listen_all(client, &ap, &settings.profile).await {
                Ok(ref _n) => debug!("crate::listen_all successful"),
                Err(e) => {
                    error!("Error: crate::listen_all reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        };

        let session_json = SessionJson(settings.session_json);
        let sqlite_store = SqliteStore(settings.sqlite_dir);

        if !ap.logout.is_none() {
            match crate::cli_logout(client, &mut ap, session_json, sqlite_store).await {
                Ok(ref _n) => debug!("crate::logout successful"),
                Err(e) => {
                    error!("Error: crate::logout reported {}", e);
                    errcount += 1;
                    result = Err(e);
                }
            };
        }
    } // end of main action block
    let plural = if errcount == 1 { "" } else { "s" };
    if errcount > 0 {
        error!("Encountered {} error{}.", errcount, plural);
    } else {
        debug!("Encountered {} error{}.", errcount, plural);
    }
    debug!("Good bye");
    result
}

/// Future test cases will be put here
#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    // for testing async functions
    // see: https://blog.x5ff.xyz/blog/async-tests-tokio-rust/
    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn test_usage() {
        assert_eq!(usage(), ());
    }

    #[test]
    fn test_help() {
        assert_eq!(help(), ());
    }

    #[test]
    fn test_manual() {
        assert_eq!(manual(), ());
    }

    #[test]
    fn test_readme() {
        assert_eq!(aw!(readme()), ());
    }

    #[test]
    fn test_version() {
        assert_eq!(version(Output::Text), ());
        assert_eq!(version(Output::Json), ());
    }

    #[test]
    fn test_version_check() {
        assert_eq!(version_check(), ());
    }

    #[test]
    fn test_contribute() {
        assert_eq!(contribute(), ());
    }

    // ─── Input validation tests (first-login data entry) ──────────────────────

    #[test]
    fn test_is_valid_username_accepts_well_formed_ids() {
        assert!(is_valid_username("@alice:matrix.org"));
        assert!(is_valid_username("@bob:example.org"));
        assert!(is_valid_username("@user123:homeserver.io"));
        assert!(is_valid_username("@very.long.name:some.server.example.com"));
    }

    #[test]
    fn test_is_valid_username_rejects_missing_at_sign() {
        assert!(!is_valid_username("alice:matrix.org"));
        assert!(!is_valid_username("bob:example.org"));
    }

    #[test]
    fn test_is_valid_username_rejects_missing_colon() {
        assert!(!is_valid_username("@alice"));
        assert!(!is_valid_username("@bob"));
    }

    #[test]
    fn test_is_valid_username_rejects_empty_string() {
        assert!(!is_valid_username(""));
    }

    #[test]
    fn test_is_valid_username_rejects_bare_at_sign() {
        // '@' with no localpart or server
        assert!(!is_valid_username("@"));
    }

    #[test]
    fn test_is_valid_username_rejects_plain_text() {
        assert!(!is_valid_username("just_a_name"));
        assert!(!is_valid_username("not a matrix id"));
    }

    #[test]
    fn test_is_valid_room_name_accepts_well_formed_ids() {
        assert!(is_valid_room_name("!roomid:matrix.org"));
        assert!(is_valid_room_name("!abc123:example.org"));
        assert!(is_valid_room_name("!aBcDeFgH:homeserver.io"));
    }

    #[test]
    fn test_is_valid_room_name_rejects_missing_exclamation_mark() {
        assert!(!is_valid_room_name("roomid:matrix.org"));
        assert!(!is_valid_room_name("abc:example.org"));
    }

    #[test]
    fn test_is_valid_room_name_rejects_missing_colon() {
        assert!(!is_valid_room_name("!roomid"));
        assert!(!is_valid_room_name("!nocolon"));
    }

    #[test]
    fn test_is_valid_room_name_rejects_two_colons() {
        // Exactly one ':' is required; two colons is invalid.
        assert!(!is_valid_room_name("!room:id:extra"));
        assert!(!is_valid_room_name("!a:b:c"));
    }

    #[test]
    fn test_is_valid_room_name_rejects_empty_string() {
        assert!(!is_valid_room_name(""));
    }

    #[test]
    fn test_is_valid_room_name_rejects_plain_text() {
        assert!(!is_valid_room_name("just_a_room"));
        assert!(!is_valid_room_name("not a room id"));
    }

    #[test]
    fn test_username_and_room_validators_are_independent() {
        // A valid user ID is not a valid room ID and vice-versa.
        assert!(is_valid_username("@alice:matrix.org"));
        assert!(!is_valid_room_name("@alice:matrix.org"));

        assert!(is_valid_room_name("!room:matrix.org"));
        assert!(!is_valid_username("!room:matrix.org"));
    }
}
