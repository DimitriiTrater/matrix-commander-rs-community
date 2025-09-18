use std::path::PathBuf;

use clap::{ColorChoice, Parser};
use matrix_sdk::ruma::{api::client::room::Visibility, OwnedMxcUri};
use url::Url;

use crate::{
    base::{consts::TIMEOUT_DEFAULT, get_prog_without_ext, get_store_default_path},
    Listen, LogLevel, LoginCLI, Logout, Output, Verify, Version,
};

#[derive(Clone, Debug, Parser, Default)]
#[command(author, version,
    next_line_help = true,
    bin_name = get_prog_without_ext(),
    color = ColorChoice::Always,
    term_width = 79,
    after_help = "PS: Also have a look at scripts/matrix-commander-rs-tui.",
    disable_version_flag = true,
    disable_help_flag = true,
)]
pub struct Args {
    #[clap(short, long, value_parser)]
    pub profile: Option<String>,
    /// Print version number or check if a newer version exists on crates.io.
    /// Details::
    /// If used without an argument such as '--version' it will
    /// print the version number. If 'check' is added ('--version check')
    /// then the program connects to https://crates.io and gets the version
    /// number of latest stable release. There is no "calling home"
    /// on every run, only a "check crates.io" upon request. Your
    /// privacy is protected. New release is neither downloaded,
    /// nor installed. It just informs you.
    #[arg(short, long, value_name = "CHECK")]
    pub version: Option<Option<Version>>,

    /// Prints a very short help summary.
    /// Details:: See also --help, --manual and --readme.
    #[arg(long)]
    pub usage: bool,

    /// Prints short help displaying about one line per argument.
    /// Details:: See also --usage, --manual and --readme.
    #[arg(short, long)]
    pub help: bool,

    /// Prints long help.
    /// Details:: This is like a man page.
    /// See also --usage, --help and --readme.
    #[arg(long)]
    pub manual: bool,

    /// Prints README.md file, the documenation in Markdown.
    /// Details:: The README.md file will be downloaded from
    /// Github. It is a Markdown file and it is best viewed with a
    /// Markdown viewer.
    /// See also --usage, --help and --manual.
    #[arg(long)]
    pub readme: bool,

    /// Overwrite the default log level.
    /// Details::
    /// If not used, then the default
    /// log level set with environment variable 'RUST_LOG' will be used.
    /// If used, log level will be set to 'DEBUG' and debugging information
    /// will be printed.
    /// '-d' is a shortcut for '--log-level DEBUG'.
    /// If used once as in '-d' it will set and/or overwrite
    /// --log-level to '--log-level debug'.
    /// If used twice as in '-d -d' it will set and/or overwrite
    /// --log-level to '--log-level debug debug'.
    /// And third or futher occurance of '-d' will be ignored.
    /// See also '--log-level'. '-d' takes precedence over '--log-level'.
    /// Additionally, have a look also at the option '--verbose'.
    #[arg(short, long,  action = clap::ArgAction::Count, default_value_t = 0u8, )]
    pub debug: u8,

    /// Set the log level by overwriting the default log level.
    /// Details::
    /// If not used, then the default
    /// log level set with environment variable 'RUST_LOG' will be used.
    /// If used with one value specified this value is assigned to the
    /// log level of matrix-commander-rs.
    /// If used with two values specified the first value is assigned to the
    /// log level of matrix-commander-rs. The second value is asigned to the
    /// lower level modules.
    /// More than two values should not be specified.
    /// --debug overwrites -log-level.
    /// See also '--debug' and '--verbose'.
    /// Alternatively you can use the RUST_LOG environment variable.
    /// An example use of RUST_LOG is to use neither --log-level nor --debug,
    /// and to set RUST_LOG="error,matrix_commander_rs=debug" which turns
    /// off debugging on all lower level modules and turns debugging on only
    /// for matrix-commander-rs.
    // Possible values are
    // '{trace}', '{debug}', '{info}', '{warn}', and '{error}'.
    #[arg(long, value_delimiter = ' ', num_args = 1..3, ignore_case = true, )]
    pub log_level: Option<Vec<LogLevel>>,

    /// Set the verbosity level.
    /// Details::
    /// If not used, then verbosity will be
    /// set to low. If used once, verbosity will be high.
    /// If used more than once, verbosity will be very high.
    /// Verbosity only affects the debug information.
    /// So, if '--debug' is not used then '--verbose' will be ignored.
    #[arg(long,  action = clap::ArgAction::Count, default_value_t = 0u8, )]
    pub verbose: u8,

    // Todo
    /// Disable encryption for a specific action.
    /// Details::
    /// By default encryption is turned on for all private rooms and DMs
    /// and turned off for all public rooms. E.g. Created DM or private room
    /// will have encryption enabled by default.
    /// To explicitly turn encryption off for a specific action use --plain.
    /// Currently --plain is supported by --room-create and --room-dm-create.
    /// See also --room-enable-encryption which sort of does the opposite for rooms.
    /// See also --visibility which allows setting the visibility of the room.
    #[arg(long)]
    pub plain: Option<bool>,

    /// Specify a path to a directory to be used as "store" for encrypted
    /// messaging.
    /// Details::
    /// Since encryption is always enabled, a store is always
    /// needed. If this option is provided, the provided
    /// directory name will be used as persistent storage
    /// directory instead of the default one. Preferably, for
    /// multiple executions of this program use the same store
    /// for the same device. The store directory can be shared
    /// between multiple different devices and users.
    #[arg(short, long,
        value_name = "PATH_TO_DIRECTORY",
        value_parser = clap::value_parser!(PathBuf),
        default_value_os_t = get_store_default_path(),
        )]
    pub store: PathBuf,

    /// Login to and authenticate with the Matrix homeserver.
    /// --login <password/sso>
    #[arg(long, value_enum,
        value_name = "LOGIN_METHOD",
        default_value_t = LoginCLI::default(), ignore_case = true, )]
    pub login: LoginCLI,

    /// Perform account verification.
    /// Details::
    /// By default, no
    /// verification is performed.
    /// Verification is currently offered via Manual-Device, Manual-User, Emoji and Emoji-Req.
    /// Do verification in this order: 1) bottstrap first with -bootstrap,
    /// 2) perform both manual verifications, and 3) perform emoji verification.
    /// --verify emoji has been tested against Element in Firefox browser and against
    /// Element app on Android phone. Both has been working successfully in Sept 2024.
    /// In Element web page it was important NOT to click the device in the device list,
    /// but to click the underscored link "Verify" just above the device list.
    /// In the Element on cell phone case, accept the emojis first on the cell phone.
    /// Manual verification is simpler but does less.
    /// Try: '--bootstrap --password mypassword --verify manual-device' or
    /// '--bootstrap --password mypassword --verify manual-user'.
    /// Manual only verfies devices or users one-directionally. See
    /// https://docs.rs/matrix-sdk/0.7/matrix_sdk/encryption/identities/struct.Device.html#method.verify
    /// and
    /// https://docs.rs/matrix-sdk/0.7/matrix_sdk/encryption/identities/struct.UserIdentity.html#method.verify
    /// for more info on Manual verification.
    /// manual-device can only verify its own devices, not other users' devices.
    /// manual-user can trust other users. So, with manual-user also use the --user option
    /// to specify one or multiple users. With manual-user first trust yourself, by
    /// setting --user to yourself, or omitting -user in which case it will default to itself.
    /// One should first do 'manual-device' and 'manual-user' verification and
    /// then 'emoji' or 'emoji-req' verification.
    /// Both 'emoji' as well as 'emoji-req' perform emoji verification.
    /// With 'emoji' we send a request to some other client to request verification from their device.
    /// With 'emoji-req' we wait for some other client to request verification from us.
    /// If verification is desired, run this program in the
    /// foreground (not as a service) and without a pipe.
    /// While verification is optional it is highly recommended, and it
    /// is recommended to be done right after (or together with) the
    /// --login action. Verification is always interactive, i.e. it
    /// required keyboard input.
    /// Verification questions
    /// will be printed on stdout and the user has to respond
    /// via the keyboard to accept or reject verification.
    /// Once verification is complete, the program may be
    /// run as a service.
    /// Different Matrix clients (like Element app on cell phone,
    /// Element website in browser, other clients) have the
    /// "Verification" button hidden in different menus or GUI
    /// elements. Sometimes it is labelled "Not trusted", sometimes "Verify"
    /// or "Verify by emoji", sometimes "Verify With Other Device".
    /// Verification is best done as follows:
    /// Run 'matrix-commander-rs --verify emoji ...' and have the
    /// program waiting for inputs and for invitations.
    /// Find the appropriate "verify" button on your other client, click it,
    /// and thereby publish a "verification invitation". Once received by
    /// "matrix-commander-rs"
    /// it will print the emojis in the terminal.
    /// At this point both your client as well as "matrix-commander-rs" in the terminal
    /// show a set of emoji icons and names. Compare them visually.
    /// Confirm on both sides (Yes, They Match, Got it), finally click OK.
    /// You should see a green shield and also see that the
    /// matrix-commander-rs device is now green and verified.
    /// In the terminal you should see a text message indicating success.
    /// It has been tested with Element app on cell phone and Element webpage in
    /// browser. Verification is done one device at a time.
    /// 'emoji-req' is similar. You must specify a user with --user and
    /// a device with --device to specify to which device you want to send the
    /// verification request. On the other device you get a pop up and you
    /// must accept the verification request.
    /// 'emoji-req' currently seems to have problems, while it does work with Element
    /// web page in browser, 'emoji-req' does not seem to
    /// work with Element phone app.
    #[arg(long, value_enum,
        value_name = "VERIFICATION_METHOD",
        default_value_t = Verify::default(), ignore_case = true, )]
    pub verify: Verify,

    // Bootstrap cross signing.
    /// Details::
    /// By default, no
    /// bootstrapping is performed. Bootstrapping is useful for verification.
    /// --bootstrap creates cross signing keys.
    /// If you have trouble verifying with --verify manual-device or
    /// --verify manual-user, use --bootstrap before.
    /// Use --password to provide password. If --password is not given it will read
    /// password from command line (stdin). See also
    /// https://docs.rs/matrix-sdk/0.7.1/matrix_sdk/encryption/struct.CrossSigningStatus.html#fields.
    #[arg(long)]
    pub bootstrap: bool,

    /// Logout this or all devices from the Matrix homeserver.
    /// Details::
    /// This requires exactly one argument.
    /// Two choices are offered: 'me' and 'all'.
    /// Provide one of these choices.
    /// If you choose 'me', only the one device "matrix-commander-rs"
    /// is currently using will be logged out.
    /// If you choose 'all', all devices of the user used by
    /// "matrix-commander-rs" will be logged out.
    /// Using '--logout all' is equivalent to
    /// '--delete-device "*" --logout "me"' and requires a password
    ///  (see --delete-device).
    /// --logout not only logs the user out from the homeserver
    /// thereby invalidates the access token, it also removes both
    /// the 'credentials' file as well as the 'store' directory.
    /// After a --logout, one must perform a new
    /// --login to use "matrix-commander-rs" again.
    /// You can perfectly use "matrix-commander-rs"
    /// without ever logging out. --logout is a cleanup
    /// if you have decided not to use this (or all) device(s) ever again.
    #[arg(long, value_enum,
        value_name = "DEVICE",
        default_value_t = Logout::default(), ignore_case = true, )]
    pub logout: Logout,

    /// Specify a homeserver for use by certain actions.
    /// Details::
    /// It is an optional argument.
    /// By default --homeserver is ignored and not used.
    /// It is used by '--login' action.
    /// If not provided for --login the user will be queried via keyboard.
    #[arg(long)]
    pub homeserver: Option<Url>,

    /// Optional argument to specify the user for --login.
    /// Details::
    /// This gives the otion to specify the user id for login.
    /// For '--login sso' the --user-login is not needed as user id can be
    /// obtained from server via SSO. For '--login password', if not
    /// provided it will be queried via keyboard. A full user id like
    /// '@john:example.com', a partial user name like '@john', and
    /// a short user name like 'john' can be given.
    /// --user-login is only used by --login and ignored by all other
    /// actions.
    #[arg(long)]
    pub user_login: Option<String>,

    /// Specify a password for use by certain actions.
    /// Details::
    /// It is an optional argument.
    /// By default --password is ignored and not used.
    /// It is used by '--login password' and '--delete-device'
    /// and --bootstrap actions.
    /// If not provided for --login, --delete-device or --bootstrap
    /// the user will be queried for the password via keyboard interactively.
    #[arg(long)]
    pub password: Option<String>,

    /// Specify a device name, for use by certain actions.
    /// Details::
    /// It is an optional argument.
    /// By default --device is ignored and not used.
    /// It is used by '--login' action.
    /// If not provided for --login the user will be queried via keyboard.
    /// If you want the default value specify ''.
    /// Multiple devices (with different device id) may have the same device
    /// name. In short, the same device name can be assigned to multiple
    /// different devices if desired
    /// Don't confuse this option with '--devices'.
    #[arg(long)]
    pub device: Option<String>,

    /// Optionally specify a room as the
    /// default room for future actions.
    /// Details::
    /// If not specified for --login, it
    /// will be queried via the keyboard. --login stores the specified room
    /// as default room in your credentials file. This option is only used
    /// in combination with --login. A default room is needed. Specify a
    /// valid room either with --room-default or provide it via keyboard.
    #[arg(long)]
    pub room_default: Option<String>,

    /// Print the list of devices.
    /// Details::
    /// All device of this
    /// account will be printed, one device per line.
    /// Don't confuse this option with --device.
    #[arg(long)]
    pub devices: bool,

    /// Set the timeout of the calls to the Matrix server.
    /// Details::
    /// By default they are set to 60 seconds.
    /// Specify the timeout in seconds. Use 0 for infinite timeout.
    #[arg(long, default_value_t = TIMEOUT_DEFAULT)]
    pub timeout: u64,

    /// Send one or more messages.
    /// Details::
    /// Message data must not be binary data, it
    /// must be text.
    // If no '-m' is used and no other conflicting
    // arguments are provided, and information is piped into the program,
    // then the piped data will be used as message.
    // Finally, if there are no operations at all in the arguments, then
    // a message will be read from stdin, i.e. from the keyboard.
    // This option can be used multiple times to send
    // multiple messages. If there is data piped
    // into this program, then first data from the
    // pipe is published, then messages from this
    // option are published. Messages will be sent last,
    // i.e. after objects like images, audio, files, events, etc.
    /// Input piped via stdin can additionally be specified with the
    /// special character '-'.
    /// If you want to feed a text message into the program
    /// via a pipe, via stdin, then specify the special
    /// character '-'.
    /// If your message is literally a single letter '-' then use an
    /// escaped '\-' or a quoted "\-".
    /// Depending on your shell, '-' might need to be escaped.
    /// If this is the case for your shell, use the escaped '\-'
    /// instead of '-' and '\\-' instead of '\-'.
    /// However, depending on which shell you are using and if you are
    /// quoting with double quotes or with single quotes, you may have
    /// to add backslashes to achieve the proper escape sequences.
    /// If you want to read the message from
    /// the keyboard use '-' and do not pipe anything into stdin, then
    /// a message will be requested and read from the keyboard.
    /// Keyboard input is limited to one line.
    /// The stdin indicator '-' may appear in any position,
    /// i.e. -m 'start' '-' 'end'
    /// will send 3 messages out of which the second one is read from stdin.
    /// The stdin indicator '-' may appear only once overall in all arguments.
    /// '-' reads everything that is in the pipe in one swoop and
    /// sends a single message.
    /// Similar to '-', another shortcut character
    /// is '_'. The special character '_' is used for
    /// streaming data via a pipe on stdin. With '_' the stdin
    /// pipe is read line-by-line and each line is treated as
    /// a separate message and sent right away. The program
    /// waits for pipe input until the pipe is closed. E.g.
    /// Imagine a tool that generates output sporadically
    /// 24x7. It can be piped, i.e. streamed, into matrix-
    /// commander, and matrix-commander stays active, sending
    /// all input instantly. If you want to send the literal
    /// letter '_' then escape it and send '\_'. '_' can be
    /// used only once. And either '-' or '_' can be used.
    #[arg(short, long, num_args(0..), )]
    pub message: Vec<String>,

    /// Specify the message format as MarkDown.
    /// Details::
    /// There are 3 message formats for '--message'.
    /// Plain text, MarkDown, and Code. By default, if no
    /// command line options are specified, 'plain text'
    /// will be used. Use '--markdown' or '--code' to set
    /// the format to MarkDown or Code respectively.
    /// '--markdown' allows sending of text
    /// formatted in MarkDown language. '--code' allows
    /// sending of text as a Code block.
    #[arg(long)]
    pub markdown: bool,

    /// Specify the message format as Code.
    /// Details::
    /// There are 3 message formats for '--message'.
    /// Plain text, MarkDown, and Code. By default, if no
    /// command line options are specified, 'plain text'
    /// will be used. Use '--markdown' or '--code' to set
    /// the format to MarkDown or Code respectively.
    /// '--markdown' allows sending of text
    /// formatted in MarkDown language. '--code' allows
    /// sending of text as a Code block.
    #[arg(long)]
    pub code: bool,

    /// Send message as format "HTML"
    /// Details::
    /// If not specified, message will be sent
    /// as format "TEXT". E.g. that allows some text
    /// to be bold, etc. Only a subset of HTML tags are
    /// accepted by Matrix.
    #[arg(long)]
    pub html: bool,

    /// Optionally specify one or multiple rooms.
    /// Details::
    /// Specify rooms via room ids or
    /// room aliases. '--room' is used by
    /// various options like '--message', '--file', some
    /// variants of '--listen', '--delete-device', etc.
    /// The default room is provided
    /// in the credentials file (specified at --login with --room-default).
    /// If a room (or multiple ones)
    /// is (or are) provided in the --room arguments, then it
    /// (or they) will be used
    /// instead of the one from the credentials file.
    /// The user must have access to the specified room
    /// in order to send messages there or listen on the room.
    /// Messages cannot
    /// be sent to arbitrary rooms. When specifying the
    /// room id some shells require the exclamation mark
    /// to be escaped with a backslash.
    // As an alternative to specifying a room as destination,
    // one can specify a user as a destination with the '--user'
    // argument. See '--user' and the term 'DM (direct messaging)'
    // for details. Specifying a room is always faster and more
    // efficient than specifying a user.
    /// Not all listen operations
    /// allow setting a room. Read more under the --listen options
    /// and similar. Most actions also support room aliases or
    /// local canonical short aliases instead of
    /// room ids. Using a room id is
    /// always faster than using a room alias.
    #[arg(short, long, num_args(0..), )]
    pub room: Vec<String>,

    /// Send one or multiple files (e.g. PDF, DOC, MP4).
    /// Details::
    /// First files are sent,
    /// then text messages are sent.
    /// If you want to feed a file into "matrix-commander-rs"
    /// via a pipe, via stdin, then specify the special
    /// character '-' as stdin indicator.
    /// See description of '--message' to see how the stdin indicator
    /// '-' is handled.
    /// If you pipe a file into stdin, you can optionally use '--file-name' to
    /// attach a label and indirectly a MIME type to the piped data.
    /// E.g. if you pipe in a PNG file, you might want to specify additionally
    /// '--file-name image.png'. As such, the label 'image' will be given
    /// to the data and the MIME type 'png' will be attached to it.
    /// Furthermore, '-' can only be used once.
    #[arg(short, long, num_args(0..), )]
    pub file: Vec<PathBuf>,

    /// Specify the message type as Notice.
    /// Details::
    /// There are 3 message types for '--message'.
    /// Text, Notice, and Emote. By default, if no
    /// command line options are specified, 'Text'
    /// will be used. Use '--notice' or '--emote' to set
    /// the type to Notice or Emote respectively.
    /// '--notice' allows sending of text
    /// as a notice. '--emote' allows
    /// sending of text as an emote.
    #[arg(long)]
    pub notice: bool,

    /// Specify the message type as Emote.
    /// Details::
    /// There are 3 message types for '--message'.
    /// Text, Notice, and Emote. By default, if no
    /// command line options are specified, 'Text'
    /// will be used. Use '--notice' or '--emote' to set
    /// the type to Notice or Emote respectively.
    /// '--notice' allows sending of text
    /// as a notice. '--emote' allows
    /// sending of text as an emote.
    #[arg(long)]
    pub emote: bool,

    /// Select synchronization choice.
    /// Details::
    /// This option decides on whether the program
    /// synchronizes the state with the server before a 'send' action.
    /// Currently two choices are offered: 'full' and 'off'.
    /// Provide one of these choices.
    /// The default is 'full'. If you want to use the default,
    /// then there is no need to use this option.
    /// If you have chosen 'full',
    /// the full state, all state events will be synchronized between
    /// this program and the server before a 'send'.
    /// If you have chosen 'off',
    /// synchronization will be skipped entirely before the 'send'
    /// which will improve performance.
    #[arg(long, value_enum,
        value_name = "SYNC_TYPE",
        default_value_t = crate::Sync::default(), ignore_case = true, )]
    pub sync: crate::Sync,

    /// Listen to messages.
    /// Details::
    /// The '--listen' option takes one argument. There are
    /// several choices: 'never', 'once', 'forever', 'tail',
    /// and 'all'. By default, --listen is set to 'never'. So,
    /// by default no listening will be done. Set it to
    /// 'forever' to listen for and print incoming messages to
    /// stdout. '--listen forever' will listen to all messages
    /// on all rooms forever. To stop listening 'forever', use
    /// Control-C on the keyboard or send a signal to the
    /// process or service.
    // The PID for signaling can be found
    // in a PID file in directory "/home/user/.run".
    /// '--listen once' will get all the messages from all rooms
    /// that are currently queued up. So, with 'once' the
    /// program will start, print waiting messages (if any)
    /// and then stop. The timeout for 'once' is set to 10
    /// seconds. So, be patient, it might take up to that
    /// amount of time. 'tail' reads and prints the last N
    /// messages from the specified rooms, then quits. The
    /// number N can be set with the '--tail' option. With
    /// 'tail' some messages read might be old, i.e. already
    /// read before, some might be new, i.e. never read
    /// before. It prints the messages and then the program
    /// stops. Messages are sorted, last-first. Look at '--tail'
    /// as that option is related to '--listen tail'. The option
    /// 'all' gets all messages available, old and new. Unlike
    /// 'once' and 'forever' that listen in ALL rooms, 'tail'
    /// and 'all' listen only to the room specified in the
    /// credentials file or the --room options.
    #[arg(short, long, value_enum,
        value_name = "LISTEN_TYPE",
        default_value_t = Listen::default(), ignore_case = true, )]
    pub listen: Listen,

    /// Get the last messages.
    /// Details::
    /// The '--tail' option reads and prints up to the last N
    /// messages from the specified rooms, then quits. It
    /// takes one argument, an integer, which we call N here.
    /// If there are fewer than N messages in a room, it reads
    /// and prints up to N messages. It gets the last N
    /// messages in reverse order. It print the newest message
    /// first, and the oldest message last. If '--listen-self'
    /// is not set it will print less than N messages in many
    /// cases because N messages are obtained, but some of
    /// them are discarded by default if they are from the
    /// user itself. Look at '--listen' as this option is
    /// related to '--tail'.
    #[arg(long, default_value_t = 0u64)]
    pub tail: u64,

    /// Get your own messages.
    /// Details::
    /// If set and listening, then program will listen to and
    /// print also the messages sent by its own user. By
    /// default messages from oneself are not printed.
    #[arg(short = 'y', long)]
    pub listen_self: bool,

    /// Print your user name.
    /// Details::
    /// Print the user id used by "matrix-commander-rs" (itself).
    /// One can get this information also by looking at the
    /// credentials file.
    #[arg(long)]
    pub whoami: bool,

    /// Output format.
    ///
    /// | Format (default: text) | Description |
    /// |--------|-------------|
    /// | `text` | Human-readable text |
    /// | `json` | JSON with minor convenience fields added |
    /// | `json-max` | JSON + `transport_response` (extra fields for `--listen`) |
    /// | `json-spec` | Strict Matrix Specification output. Only works with `--listen`/`--tail` |
    ///
    /// **Note:** `json-spec` produces no output for commands like `--get-room-info`.
    #[arg(short, long, value_enum,
        value_name = "OUTPUT_FORMAT",
        default_value_t = Output::default(), ignore_case = true, )]
    pub output: Output,

    /// Specify one or multiple file names for some actions.
    /// Details::
    /// This is an optional argument. Use this option in
    /// combination with options like '--file'.
    // or '--download'
    /// to specify
    /// one or multiple file names. Ignored if used by itself
    /// without an appropriate corresponding action.
    #[arg(long, num_args(0..), )]
    pub file_name: Vec<PathBuf>,

    /// Get room information.
    /// Details::
    /// Get the room information such as room display name,
    /// room alias, room creator, etc. for one or multiple
    /// specified rooms. The included room 'display name' is
    /// also referred to as 'room name' or incorrectly even as
    /// room title. If one or more rooms are given, the room
    /// information of these rooms will be fetched. If no
    /// room is specified, nothing will be done.
    /// If you want the room information for the
    /// pre-configured default room specify the shortcut '-'.
    /// Rooms can be given via room id (e.g.
    /// '\!SomeRoomId:matrix.example.com'), canonical (full)
    /// room alias (e.g. '#SomeRoomAlias:matrix.example.com'),
    /// or short alias (e.g. 'SomeRoomAlias' or
    /// '#SomeRoomAlias').
    /// As response room id, room display
    /// name, room canonical alias, room topic, room creator,
    /// and room encryption are printed. One line per room
    /// will be printed.
    /// Since either room id or room alias
    /// are accepted as input and both room id and room alias
    /// are given as output, one can hence use this option to
    /// map from room id to room alias as well as vice versa
    /// from room alias to room id.
    /// Do not confuse this option
    /// with the options '--get-display-name' and
    /// '--set-display-name', which get/set the user display name,
    /// not the room display name.
    /// The argument '--room-resolve-alias' can also be used
    /// to go the other direction, i.e. to find the room id
    /// given a room alias.
    #[arg(long, num_args(0..), value_name = "ROOM",
        alias = "room-get-info")]
    pub get_room_info: Vec<String>,

    /// Create one or multiple rooms.
    /// Details::
    /// One or multiple room
    /// aliases can be specified. For each alias specified a
    /// room will be created. For each created room one line
    /// with room id, alias, name and topic will be printed
    /// to stdout. If
    /// you are not interested in an alias, provide an empty
    /// string like ''. The alias provided must be in canocial
    /// local form, i.e. if you want a final full alias like
    /// '#SomeRoomAlias:matrix.example.com' you must provide
    /// the string 'SomeRoomAlias'. The user must be permitted
    /// to create rooms. Combine --room-create with --name and
    /// --topic to add names and topics to the room(s) to be
    /// created.
    /// If the output is in JSON format, then the values that
    /// are not set and hence have default values are not shown
    /// in the JSON output. E.g. if no topic is given, then
    /// there will be no topic field in the JSON output.
    /// Room aliases have to be unique.
    #[arg(long, num_args(0..), value_name = "LOCAL_ALIAS", )]
    pub room_create: Vec<String>,

    /// Set the visibility of the newly created room.
    /// Details::
    /// Default room visibility is 'private'.
    /// To create a public room, use
    /// '--room-create <room-name> --visibility public'.
    /// To create a private room, use
    /// '--room-create <room-name> --visibility private'.
    #[arg(long, value_enum,
        value_name = "VISIBILITY",
        default_value = Visibility::Private.as_str(), ignore_case = true, )]
    pub visibility: Visibility,

    /// Create one or multiple direct messaging (DM) rooms
    /// for given users.
    /// Details::
    /// One or multiple
    /// users can be specified. For each user specified a
    /// DM room will be created. For each created DM room one line
    /// with room id, alias, name and topic will be printed
    /// to stdout. The given user(s) will receive an invitation
    /// to join the newly created room.
    /// The user must be permitted
    /// to create rooms. Combine --room-dm-create with --alias,
    /// --name and
    /// --topic to add aliases, names and topics to the room(s) to be
    /// created.
    // If the output is in JSON format, then the values that
    // are not set and hence have default values are not shown
    // in the JSON output. E.g. if no topic is given, then
    // there will be no topic field in the JSON output.
    /// Room aliases in --alias have to be unique.
    #[arg(long, num_args(0..), value_name = "USER", )]
    pub room_dm_create: Vec<String>,

    /// Leave this room or these rooms.
    /// Details::
    /// One or multiple room
    /// aliases can be specified. The room (or multiple ones)
    /// provided in the arguments will be left.
    /// You can run both commands '--room-leave' and
    /// '--room-forget' at the same time
    #[arg(long, num_args(0..), value_name = "ROOM", )]
    pub room_leave: Vec<String>,

    /// Forget one or multiple rooms.
    /// Details::
    /// After leaving a room you should (most likely) forget
    /// the room. Forgetting a room removes the users' room
    /// history. One or multiple room aliases can be
    /// specified. The room (or multiple ones) provided in the
    /// arguments will be forgotten. If all users forget a
    /// room, the room can eventually be deleted on the
    /// server. You must leave a room first, before you can
    /// forget it
    /// You can run both commands '--room-leave' and
    /// '--room-forget' at the same time
    #[arg(long, num_args(0..), value_name = "ROOM", )]
    pub room_forget: Vec<String>,

    /// Invite one ore more users to join one or more rooms.
    /// Details::
    /// Specify the user(s) as arguments to --user. Specify
    /// the rooms as arguments to this option, i.e. as
    /// arguments to --room-invite. The user must have
    /// permissions to invite users.
    /// Use the shortcut '-' to specify the pre-configured
    /// default room of 'matrix-commander-rs' as room.
    #[arg(long, num_args(0..), value_name = "ROOM", )]
    pub room_invite: Vec<String>,

    /// Join one or multiple rooms.
    /// Details::
    /// One or multiple room
    /// aliases can be specified. The room (or multiple ones)
    /// provided in the arguments will be joined. The user
    /// must have permissions to join these rooms.
    /// Use the shortcut '-' to specify the pre-configured
    /// default room of 'matrix-commander-rs' as room.
    /// Note, no --user on this feature as the user is
    /// always the user of 'matrix-commander-rs'.
    #[arg(long, num_args(0..), value_name = "ROOM", )]
    pub room_join: Vec<String>,

    /// Ban one ore more users from one or more rooms.
    /// Details::
    /// Specify
    /// the user(s) as arguments to --user. Specify the rooms
    /// as arguments to this option, i.e. as arguments to
    /// --room-ban. The user must have permissions to ban
    /// users.
    /// Use the shortcut '-' to specify the pre-configured
    /// default room of 'matrix-commander-rs' as room.
    #[arg(long, num_args(0..), value_name = "ROOM", )]
    pub room_ban: Vec<String>,

    /// Unban one ore more users from one or more rooms.
    /// Details::
    /// Specify the user(s) as arguments to --user. Specify
    /// the rooms as arguments to this option, i.e. as
    /// arguments to --room-unban. The user must have
    /// permissions to unban users.
    /// Use the shortcut '-' to specify the pre-configured
    /// default room of 'matrix-commander-rs' as room.
    /// Note, this is currently not implemented in the
    /// matrix-sdk API. This feature will currently return
    /// an error.
    #[arg(long, num_args(0..), value_name = "ROOM", )]
    pub room_unban: Vec<String>,

    /// Kick one ore more users from one or more rooms.
    /// Details::
    /// Specify the user(s) as arguments to --user. Specify
    /// the rooms as arguments to this option, i.e. as
    /// arguments to --room-kick. The user must have
    /// permissions to kick users.
    /// Use the shortcut '-' to specify the pre-configured
    /// default room of 'matrix-commander-rs' as room.
    #[arg(long, num_args(0..), value_name = "ROOM", )]
    pub room_kick: Vec<String>,

    /// Resolves room aliases to room ids.
    /// Details::
    /// Resolves a room alias to the corresponding room id, or
    /// multiple room aliases to their corresponding room ids.
    /// Provide one or multiple room aliases. A room alias
    /// looks like this: '#someRoomAlias:matrix.example.org'.
    /// Short aliases like 'someRoomAlias' or '#someRoomAlias'
    /// are also accepted. In case of a short alias, it will
    /// be automatically prefixed with '#' and the homeserver
    /// from the default room of matrix-commander-rs (as found in
    /// credentials file) will be automatically appended.
    /// Resolving an alias that does not exist results in an
    /// error. For each room alias one line will be printed to
    /// stdout with the result. It also prints the list of
    /// servers that know about the alias(es).
    /// The argument '--get-room-info' can be used to go the
    /// other direction, i.e. to find the room aliases
    /// given a room id.
    #[arg(long, num_args(0..), value_name = "ALIAS", )]
    pub room_resolve_alias: Vec<String>,

    /// Enable encryption for one or multiple rooms.
    /// Details::
    /// Provide one or more room ids. For each room given
    /// encryption will be enabled. You must be member of the
    /// room in order to be able to enable encryption. Use
    /// shortcut '-' to enable encryption in the pre-configured
    /// default room. Enabling an already enabled room will
    /// do nothing and cause no error.
    #[arg(long, num_args(0..), value_name = "ROOM", )]
    pub room_enable_encryption: Vec<String>,

    /// Provide one or more aliases.
    /// Details::
    /// --alias is currently used in
    /// combination with --room-dm-create. It is ignored otherwise.
    /// Canonical short alias look like 'SomeRoomAlias'.
    /// Short aliases look like '#SomeRoomAlias'. And full aliases
    /// look like '#SomeRoomAlias:matrix.example.com'.
    /// If you are not interested in an alias, provide an empty
    /// string like ''. Remember that aliases must be unique. For
    /// --room-dm-create you must provide canonical short alias(es).
    #[arg(long, num_args(0..), value_name = "ALIAS", )]
    pub alias: Vec<String>,

    /// Specify one or multiple names.
    /// Details::
    /// This option is only
    /// meaningful in combination with option --room-create.
    /// This option --name specifies the names to be used with
    /// the command --room-create.
    #[arg(long, num_args(0..), )]
    pub name: Vec<String>,

    /// Specify one or multiple topics.
    /// Details::
    /// This option is only
    /// meaningful in combination with option --room-create.
    /// This option --topic specifies the topics to be used
    /// with the command --room-create.
    #[arg(long, num_args(0..), )]
    pub topic: Vec<String>,

    /// Print the list of past and current rooms.
    /// Details::
    /// All rooms that you
    /// are currently a member of (joined rooms), that you had been a
    /// member of in the past (left rooms), and rooms that you have
    /// been invited to (invited rooms) will be printed,
    /// one room per line. See also '--invited-rooms',
    /// '--joined-rooms', and '--left-rooms'.
    #[arg(long)]
    pub rooms: bool,

    /// Print the list of invited rooms.
    /// Details::
    /// All rooms that you are
    /// currently invited to will be printed, one room per line.
    #[arg(long)]
    pub invited_rooms: bool,

    /// Print the list of joined rooms.
    /// Details::
    /// All rooms that you are
    /// currently a member of will be printed, one room per line.
    #[arg(long)]
    pub joined_rooms: bool,

    /// Print the list of left rooms.
    /// Details::
    /// All rooms that you have
    /// left in the past will be printed, one room per line.
    #[arg(long)]
    pub left_rooms: bool,

    /// Get the visibility of one or more rooms.
    /// Details::
    /// Provide one
    /// or more room ids as arguments. If the shortcut '-' is
    /// used, then the default room of 'matrix-commander-rs' (as
    /// found in credentials file) will be used. The shortcut
    /// '*' represents all the rooms of the user of
    /// 'matrix-commander-rs'.
    /// For each room
    /// the visibility will be printed. Currently, this is
    /// either the string 'private' or 'public'. As response
    /// one line per room will be printed.
    #[arg(long, num_args(0..), alias = "get_room_visibility",
        value_name = "ROOM", )]
    pub room_get_visibility: Vec<String>,

    /// Get the state of one or more rooms.
    /// Details::
    /// Provide one or
    /// more room ids as arguments. If the shortcut '-' is
    /// used, then the default room of 'matrix-commander-rs' (as
    /// found in credentials file) will be used. The shortcut
    /// '*' represents all the rooms of the user of
    /// 'matrix-commander-rs'.
    /// For each room part of the
    /// state will be printed. The state is a long list of
    /// events. As
    /// response one line per room will be printed to stdout.
    /// The line can be very long as the list of events can be
    /// very large. To get output into a human readable form
    /// pipe output through sed and jq or use the JSON output.
    #[arg(long, num_args(0..), alias = "get_room_state",
        value_name = "ROOM", )]
    pub room_get_state: Vec<String>,

    /// Print the list of joined members for one or multiple
    /// rooms.
    /// Details::
    /// If you want to print the joined members of all
    /// rooms that you are member of, then use the special
    /// shortcut character '*'. If you want the members of
    /// the pre-configured default room, use shortcut '-'.
    #[arg(long, num_args(0..), value_name = "ROOM", )]
    pub joined_members: Vec<String>,

    /// Delete one or multiple devices.
    /// Details::
    /// By default devices
    /// belonging to itself, i.e. belonging to
    /// "matrix-commander-rs", will be deleted.
    /// If you want to delete the one device
    /// currently used for the connection, i.e. the device
    /// used by "matrix-commander-rs", then instead of the
    /// full device id you can just specify the shortcut 'me'
    /// such as '--delete-device me --password mypassword'.
    /// If you want to delete all devices of yourself, i.e.
    /// all devices owned by the user that
    /// "matrix-commander-rs" is using you can specify
    /// that with the shortcut '*'. Most shells require you
    /// to escape it or to quote it, ie. use
    /// '--delete-device "*" --password mypassword'.
    /// Removing your own device (e.g. 'me') or all devices
    /// (e.g. '*') will require you to manually remove your
    /// credentials file and store directory and to login
    /// anew in order to create a new device.
    /// If you are using
    /// '--delete-device me --password mypassword' consider
    /// using '--logout me' instead which is simpler
    /// (no password) and also automatically performs the
    /// removal of credentials and store. (See --logout.)
    /// If the devices belong to a different user, use the --user
    /// argument to specify the user, i.e. owner. Only exactly
    /// one user can be specified with the optional --user
    /// argument. Device deletion requires the user password.
    /// It must be specified with the --password argument. If
    /// the server uses only HTTP (and not HTTPS), then the
    /// password can be visible to attackers. Hence, if the
    /// server does not support HTTPS this operation is
    /// discouraged.
    /// If no --password is specified via the command line,
    /// the password is read from keyboard interactively.
    #[arg(long, num_args(0..),
        value_name = "DEVICE", )]
    pub delete_device: Vec<String>,

    /// Specify one or multiple users.
    /// Details::
    /// This option is
    /// meaningful in combination with
    /// a) room actions like
    /// --room-invite, --room-ban, --room-unban, etc. and
    // b)
    // send actions like -m, -i, -f, etc. c) some listen
    // actions --listen, as well as
    /// d) actions like
    /// --delete-device.
    /// In case of a) this option --user specifies the
    /// users to be used with room commands (like invite, ban,
    // etc.).
    // In case of b) the option --user can be used as
    // an alternative to specifying a room as destination for
    // text (-m), images (-i), etc. For send actions '--user'
    // is providing the functionality of 'DM (direct
    // messaging)'. For c) this option allows an alternative
    // to specifying a room as destination for some --listen
    // actions.
    /// For d) this gives the option to delete the
    /// device of a different user.
    // ----- What is a DM?
    // matrix-commander tries to find a room that contains
    // only the sender and the receiver, hence DM. These
    // rooms have nothing special other the fact that they
    // only have 2 members and them being the sender and
    // recipient respectively. If such a room is found, the
    // first one found will be used as destination. If no
    // such room is found, the send fails and the user should
    // do a --room-create and --room-invite first. If
    // multiple such rooms exist, one of them will be used
    // (arbitrarily). For sending and listening, specifying a
    // room directly is always faster and more efficient than
    // specifying a user. So, if you know the room, it is
    // preferred to use --room instead of --user. For b) and
    // c) --user can be specified in 3 ways: 1) full user id
    // as in '@john:example.org', 2) partial user id as in
    // '@john' when the user is on the same homeserver
    // (example.org will be automatically appended), or 3) a
    // display name as in 'john'. Be careful, when using
    // display names as they might not be unique, and you
    // could be sending to the wrong person. To see possible
    // display names use the --joined-members '*' option
    // which will show you the display names in the middle
    // column.
    /// If --user is not set, it will default to itself,
    /// i.e. the user of the "matrix-commander-rs" account.
    #[arg(short, long, num_args(0..), )]
    pub user: Vec<String>,

    /// Get your own avatar.
    /// Details::
    /// Get the avatar of itself, i.e. the
    /// 'matrix-commander-rs' user account. Spefify a
    /// file optionally with path to store the image.
    /// E.g. --get-avatar "./avatar.png".
    #[arg(long, value_name = "FILE")]
    pub get_avatar: Option<PathBuf>,

    /// Set your own avatar.
    /// Details::
    /// Set, i.e. upload, an image to be used as avatar for
    /// 'matrix-commander-rs' user account. Spefify a
    /// file optionally with path with the image. If the MIME
    /// type of the image cannot be determined, it will
    /// assume 'PNG' as default.
    /// E.g. --set-avatar "./avatar.jpg".
    /// It returns a line with the MRX URI of the new
    /// avatar.
    #[arg(long, alias = "upload-avatar", value_name = "FILE")]
    pub set_avatar: Option<PathBuf>,

    /// Get your own avatar URL.
    /// Details::
    /// Get the MXC URI of the avatar of itself, i.e. the
    /// 'matrix-commander-rs' user account.
    #[arg(long)]
    pub get_avatar_url: bool,

    /// Set your own avatar URL.
    /// Details::
    /// Set the avatar MXC URI of the URL to be used as avatar for
    /// the 'matrix-commander-rs' user account. Spefify a
    /// MXC URI.
    /// E.g. --set-avatar-url "mxc://matrix.server.org/SomeStrangeStringOfYourMxcUri".
    #[arg(long, alias = "upload-avatar-url", value_name = "MAX_URI")]
    pub set_avatar_url: Option<OwnedMxcUri>,

    /// Remove your own avatar URL.
    /// Details::
    /// Remove the avatar MXC URI to be used as avatar for
    /// the 'matrix-commander-rs' user account. In other words, remove
    /// the avatar of the 'matrix-commander-rs' user.
    #[arg(long, alias = "remove-avatar")]
    pub unset_avatar_url: bool,

    /// Get your own display name.
    /// Details::
    /// Get the display name of itself, i.e. of the
    /// 'matrix-commander-rs' user account.
    #[arg(long)]
    pub get_display_name: bool,

    /// Set your own display name.
    /// Details::
    /// Set the display name of
    /// the 'matrix-commander-rs' user account. Spefify a
    /// name.
    #[arg(long, value_name = "NAME")]
    pub set_display_name: Option<String>,

    /// Get your own profile.
    /// Details::
    /// Get the profile of itself, i.e. of the
    /// 'matrix-commander-rs' user account. This is
    /// getting both display name and avatar MXC URI in a call.
    #[arg(long)]
    pub get_profile: bool,

    /// Upload one or multiple files (e.g. PDF, DOC, MP4) to the
    /// homeserver content repository.
    /// Details::
    /// If you want to feed a file for upload into "matrix-commander-rs"
    /// via a pipe, via stdin, then specify the special
    /// character '-' as stdin indicator.
    /// See description of '--message' to see how the stdin indicator
    /// '-' is handled. Use --mime to optionally specify the MIME type
    /// of the file. If you give N arguments to --media-upload, you
    /// can give N arguments to --mime. See --mime.
    /// If you pipe a file into stdin, the MIME type cannot be guessed.
    /// It is hence more recommended that you specify a MIME type via
    /// '--mime' when using '-'.
    /// Furthermore, '-' can only be used once.
    /// Upon being stored in the homeserver's content repository, the
    /// data is assigned a Matrix MXC URI. For each file uploaded
    /// successfully, a
    /// single line with the MXC URI will be printed.
    /// The uploaded data will not by encrypted.
    /// If you want to upload encrypted data, encrypt the file before
    /// uploading it.
    // Use --plain to disable encryption for the upload.
    #[arg(long, alias = "upload", value_name = "FILE", num_args(0..), )]
    pub media_upload: Vec<PathBuf>,

    /// Download one or multiple files from the homeserver content
    /// repository.
    /// Details::
    /// You must provide one or multiple Matrix
    /// URIs (MXCs) which are strings like this
    /// 'mxc://example.com/SomeStrangeUriKey'.
    /// Alternatively,
    /// you can just provide the MXC id, i.e. the part after
    /// the last slash.
    /// If found they
    /// will be downloaded, decrypted, and stored in local
    /// files. If file names are specified with --file-name
    /// the downloads will be saved with these file names. If
    /// --file-name is not specified, then the file name
    /// 'mxc-<mxc-id>' will be used. If a file name in
    /// --file-name
    /// contains the placeholder __mxc_id__, it will be
    /// replaced with the mxc-id. If a file name is specified
    /// as empty string '' in --file-name, then also the name
    /// 'mxc-<mxc-id>' will be used. Be careful, existing
    /// files will be overwritten.
    // By default, the upload
    // was encrypted so a decryption dictionary must be
    // provided to decrypt the data. Specify one or multiple
    // decryption keys with --key-dict. If --key-dict is not
    // set, no decryption is attempted; and the data might
    // be stored in encrypted fashion, or might be plain-text
    // if the file was uploaded in plain text.
    // ....if the --upload skipped encryption with --plain. See
    // tests/test-upload.sh for an example.
    /// Do not confuse --media-download with --download-media.
    /// See --download-media.
    #[arg(long, alias = "download", value_name = "MXC_URI", num_args(0..), )]
    pub media_download: Vec<OwnedMxcUri>,

    /// Specify the Mime type of certain input files.
    /// Details::
    /// Specify '' if the Mime type should be guessed
    /// based on the filename. If input is from stdin
    /// (i.e. '-' and piped into 'matrix-commander-rs')
    /// then Mime type cannot be guessed. If not specified,
    /// and no filename available for guessing it will
    /// default to 'application/octet-stream'. Some example
    /// mime types are: 'image/jpeg', 'image/png', 'image/gif',
    /// 'text/plain', and 'application/pdf'. For a full
    /// list see 'https://docs.rs/mime/latest/mime/#constants'.
    // One cannot use Vec<Mime> as type because that prevents '' from being used.
    #[arg(long, value_name = "MIME_TYPE", num_args(0..), )]
    pub mime: Vec<String>,

    /// Delete one or multiple objects (e.g. files) from the
    /// content repository.
    /// Details::
    /// You must provide one or multiple
    /// Matrix URIs (MXC) which are strings like this
    /// 'mxc://example.com/SomeStrangeUriKey'. Alternatively,
    /// you can just provide the MXC id, i.e. the part after
    /// the last slash. If found they will be deleted from the
    /// server database. In order to delete objects one must
    /// have server admin permissions. Having only room admin
    /// permissions is not sufficient and it will fail. Read
    /// https://matrix-org.github.io/synapse/latest/usage/administration/admin_api/
    /// for learning how to set server
    /// admin permissions on the server.
    /// Thumbnails will currently not be deleted.
    /// Deleting something that does not exist will be ignored
    /// and will not cause an error.
    #[arg(long, alias = "delete-mxc", value_name = "MXC_URI", num_args(0..), )]
    pub media_delete: Vec<OwnedMxcUri>,

    /// Convert URIs to HTTP URLs.
    /// Details::
    /// Convert one or more matrix content URIs to the
    /// corresponding HTTP URLs. The MXC URIs to provide look
    /// something like this
    /// 'mxc://example.com/SomeStrangeUriKey'.
    /// Alternatively,
    /// you can just provide the MXC id, i.e. the part after
    /// the last slash.
    /// The syntax of the provided MXC URIs will be verified.
    /// The existance of content for the XMC URI will not be checked.
    // This works without a server or without being logged in.
    #[arg(long, alias = "mxc-to-http", value_name = "MXC_URI", num_args(0..), )]
    pub media_mxc_to_http: Vec<OwnedMxcUri>,

    /// Get your own master key.
    /// Details::
    /// Get the master key of itself, i.e. of the
    /// 'matrix-commander-rs' user account. Keep
    /// this key private and safe.
    #[arg(long)]
    pub get_masterkey: bool,

    /// Generated default config in .config/mcrs
    #[arg(long, short)]
    pub config: bool,
}
