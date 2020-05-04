# last-date

Simple ToDo list based on last date I did.

![index page image](./img/index.jpg)

## Purpose

This app's purpose is to remind **me** of forgotten daily tasks like cleaning in specific places (window, shoe case, bed cover, ...).

## Usage

See [usage.md](./usage.md) file.

## Run

First, we have to install `sqlite3`.

We have to use **nightly** Rust because Web framework [Rocket](https://rocket.rs/) uses it.

```bash
git clone https://github.com/yammmt/last-date.git
cd last-date
rustup override set nightly
cargo run
```

You can access your site by accessing `http://localhost:8000`.

### Production environment

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

If you want to work with DB directly, you can use SQLite3 (`sqlite3 db/dev.sqlite`).

```sql
sqlite> INSERT INTO tasks(name,description,updated_at) VALUES ("Eat sushi","Essential to my life!","2020-04-20");
sqlite> SELECT * FROM tasks;
110|Eat sushi|Essential to my life!|2020-04-20
```

## Links

- [Rocket todo example](https://github.com/SergioBenitez/Rocket/tree/master/examples/todo)
    - This app is based on this example :bow:
- CSS framework [Bulma](https://bulma.io/)
