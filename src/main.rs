#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
extern crate log;
#[macro_use]
extern crate serde_derive;

mod models;
mod routes;
#[cfg(test)]
mod tests;

use crate::diesel::connection::SimpleConnection;
use rocket::fairing::AdHoc;
use rocket::fs::{FileServer, relative};
use rocket::{Build, Rocket};
use rocket_dyn_templates::Template;
use rocket_sync_db_pools::database;

#[database("sqlite_database")]
pub struct DbConn(diesel::SqliteConnection);

async fn run_db_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
    let conn = DbConn::get_one(&rocket).await.expect("database connection");

    conn.run(|conn| {
        conn.run_pending_migrations(MIGRATIONS)
            .expect("diesel migrations");
    })
    .await;

    conn.run(|conn| {
        conn.batch_execute("PRAGMA foreign_keys = ON")
            .expect("Failed to enable foreign keys")
    })
    .await;

    rocket
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(DbConn::fairing())
        .attach(AdHoc::on_ignite("Database Migrations", run_db_migrations))
        .attach(Template::fairing())
        .mount("/", FileServer::from(relative!("static")))
        .mount(
            "/",
            routes![
                routes::task::index,
                routes::task::new,
                routes::task::update_date,
                routes::task::update,
                routes::task::edit,
                routes::task::delete,
                routes::task::confirm,
                routes::task::by_label,
                routes::label::index,
                routes::label::new,
                routes::label::update,
                routes::label::edit,
                routes::label::confirm,
                routes::label::delete
            ],
        )
}
