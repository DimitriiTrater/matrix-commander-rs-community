[![crates.io - Version](
https://img.shields.io/crates/v/matrix-commander
)](https://crates.io/crates/matrix-commander)
[![crates.io - Downloads](
https://img.shields.io/crates/d/matrix-commander
)](https://crates.io/crates/matrix-commander)

<p>
<img
src="https://raw.githubusercontent.com/DimitriiTrater/matrix-commander-rs-community/master/logos/matrix-commander-rs.svg"
alt="MC logo" height="150">

# matrix-commander-rs-community
This project is based on matrix-commander-rs by 8go
licensed under GPL v3.
This is an independently maintained continuation of the project.

# Help create this Rust program

This Rust project is currently bare essentials. 
A more feature-rich Python package `matrix-commander`
exists. See [matrix-commander](https://github.com/8go/matrix-commander/).
The vision is to have a compatible program in Rust.
This project depends on you. The project will only advance if you provide
some code. Have a look at the repo
[matrix-commander-rs](https://github.com/8go/matrix-commander-rs/).
Please help! :pray: Please contribute code to enhance this
[matrix-commander](https://crates.io/crates/matrix-commander) crate.
Safe!

:heart: :clap: :pray:

# What works so far

- Login with password
- Login with access token (restore login)
- Encryption
- Manual and Emoji verification
- Sending one or multiple text message to one or multiple rooms
- Sending one or multiple text message to one or multiple rooms
- Listening for new and incoming messages on one or multiple rooms
- Getting and printing old messages
- Listing devices
- Creating, leaving and forgetting rooms
- Kicking, banning, etc. on rooms
- Getting, setting and removing user avatar
- Getting room info
- Logout and removal of device
- Things like argument parsing, logging, output in JSON format, etc.
- Creating a brand new client, sending a message and destroying the client
  all in a single command. This send-and-forget command is:
  `matrix-commander-rs --login password --user-login @john:some.homeserver.org
  --password secret --device matrix-commander-rs --room-default
  \!someRoomId:some.homeserver.org --message Hello --logout me`.

# What you can do

- Give a :star: on Github. The more stars on Github, the more people will
  see the project. Do it now, thanks. :clap:
- Talk about it to your friends, post it in chatrooms, Hacker News, etc.
  This will give exposure and help find people willing to provide code,
  contributions, and PRs.
- Write code yourself. :rocket: Features that you might want to code: 
  - implement `login` via SSO
  - add --proxy (see Python documentation)
  - add --nossl (see Python documentation)
  - add --event (see Python documentation and JSON config file in Pythom repo)
  - add --download-media (see Python documentation)
  - add other features found in the Python version to the Rust version
  - ...
