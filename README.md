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

### For developer

To use [Diesel](https://github.com/diesel-rs/diesel) from command line, diesel-cli is needed.

```bash
cargo install diesel_cli --no-default-features --features sqlite
export DATABASE_URL=db/mydb.sqlite # if needed
```

Use clippy for code lint.
```bash
cargo clippy
```

## Links

- [Rocket todo example](https://github.com/SergioBenitez/Rocket/tree/master/examples/todo)
- CSS framework [Bulma](https://bulma.io/)
