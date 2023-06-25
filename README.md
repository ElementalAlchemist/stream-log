# Stream Log
A web application designed to help large teams document events that occur during a livestreamed event. It features a way
for a team to record detailed occurrences in a web interface with live updates and an API for integration with a video
upload management system.

## Installation & Running
1. Get the code.
Download a release of Stream Log. If you use `git`, it's recommended that you check out the most recent release tag. If
you do this, ensure that you check out the next tag when you upgrade the program.

2. Ensure you can compile Rust programs.
You'll need a Rust compiler on your system. The generally recommended way to do this is using
[rustup](https://www.rust-lang.org/tools/install), a tool that can help manage Rust installations, keep them up to date,
and allow the appropriate targets to be installed. If you can't install system-wide, `rustup` can be installed for a
single user. Keeping the Rust compiler updated is important—Stream Log frequently updates its minimum supported Rust
version to take advantage of new features in new versions of Rust.

Stream Log compiles on the stable version of Rust, so the stable toolchain will be sufficient. The compile target for
your system is installed by default. The client code needs to compile to WebAssembly, so you'll also need to install the
`wasm32-unknown-unknown` target using `rustup target install wasm32-unknown-unknown`.

3. Install the client web builder.
The client is built using a tool that has extra features for building web applications. For this, you'll need to install
[Trunk](https://trunkrs.dev).

4. Create a PostgreSQL database.
Stream Log needs to have a user and database in a PostgreSQL instance. For the initial run and on upgrade, Stream Log
will need permissions to create tables and update the database schema. If you'd like, you can restrict these permissions
on subsequent runs. (More information will be provided about this later.)

5. Set up a web server proxy.
Stream Log is designed to work on a server proxied through an existing web server. This program does not handle its own
TLS connections. Note that Stream Log requires an HTTP 1.1 connection, so if you have a web server that defaults its
reverse proxy configuration to use HTTP 1.0 (as nginx does), you'll need to specify that explicitly (for nginx,
`proxy_http_version 1.1`).

6. Compile the client.
In the `client` directory, run `trunk build --release`. If you won't be hosting Stream Log at the root of your domain
name (e.g. if you'll be hosting at https://example.com/stream-log instead of https://stream-log.example.com or
https://example.com), you must specify trunk's `--public-url` option so that the links point to the right locations (in
this example, `trunk build --release --public-url stream-log` or
`trunk build --release --public-url https://example.com/stream-log`).

7. Point the server to the client.
In the `server` directory, create a link named `static` to `client/dist`. On a Linux or Unix system, you could do this
by running `ln -s ../client/dist static`.

8. Configure the server.
In the `server` directory, copy `config-example.kdl` to `config.kdl` and follow the instructions in the example
configuration file to configure the server.

9. Run the server.
You can compile and run the server using `cargo run --release`. If you want to run just database migrations (for initial
setup or for upgrades of Stream Log), you can run `cargo run --release -- --migrations-only`. If you want to restrict
database permissions during normal runtime of the system, you can grant permissions to modify the database schema, run
Stream Log with the `--migrations-only` flag, and then revoke those permissions again, or you could configure Stream Log
with a different database user with the appropriate permissions for only the `--migrations-only` run.

10. Do initial user creation.
If you haven't run Stream Log before and the database is empty, the first user to be registered in the system is
automatically made an administrator. Once you're registered and signed in, you can set up the system using the
administrative features in the admin menu.