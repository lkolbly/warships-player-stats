World of Warships Statistics Server
===================================

Welcome! If you just want to see this server in action, you can check it out here: https://pillow.rscheme.org/warshipstats/player/lkolbly (substitute your own username into the URL)

Of course, you're welcome to set it up yourself.

Setup
=====

I'm going to assume you're using a Linux machine (I use Ubuntu). I'm not aware of anything that's explicitly Windows-specific, though.

1. You will need a Mongo DB server somewhere to host the data. The North America server dataset takes up approximately 10GB of space.
2. You will need a World of Warships API key. You can get one from https://developers.wargaming.net
3. Create a `settings.toml` file by copying `settings.toml.example` and plugging in your API key and mongo URL.
4. Extract `GameParams.data` from the game files, and convert it into a `GameParams.json` file using [WoWS-GameParams](https://github.com/EdibleBug/WoWS-GameParams). Copy that `GameParams.json` file to where you will run the server, along with your `settings.toml`.
5. Install [Rust](https://www.rust-lang.org/), if you haven't already.
6. In this directory, run `cargo build --release`.
7. Run the generated `./target/release/wows-player-stats` executable. It should automatically start pulling from the API and filling up the database.
8. Enjoy!

Contributing
============

If you find any issues, or want any features, feel free to open an issue or make a PR!
