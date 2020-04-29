# last-date

## Run

First, we have to install `sqlite3`.

We have to use nightly Rust because Web framework [Rocket](https://rocket.rs/) uses it.

```bash
git clone https://github.com/yammmt/last-date.git
cd last-date
rustup override set nightly
cargo run
```

### As production

If you want to run this in production environment, for example, run following commands.
```bash
export ROCKET_SECRET_KEY=<your secret key>
export ROCKET_ENV=production
cargo run --release
```

You can access your site by accessing `http://<your machine address>:8000`.  
Note that you can generate secret key with `openssl rand -base64 32`.

### For developer

To use [Diesel](https://github.com/diesel-rs/diesel) from command line, diesel-cli is needed.

```bash
cargo install diesel_cli --no-default-features --features sqlite
export DATABASE_URL=db/mydb.sqlite # if needed
```

To use database file different from one defined in `Rocket.toml`, set environment variable.
```bash
export ROCKET_DATABASES='{sqlite_database={url="db/mydb.sqlite"}}'
```
Note that this setting is common between `cargo run` and `cargo test`.

Use clippy for code lint.
```bash
cargo clippy
```

## Links

- [Rocket todo example](https://github.com/SergioBenitez/Rocket/tree/master/examples/todo)
- CSS framework [Bulma](https://bulma.io/)
