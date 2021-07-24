#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

mod models;
mod routes;
#[cfg(test)] mod tests;

use diesel::Connection;
use rocket::{Build, Rocket};
use rocket::fairing::AdHoc;
use rocket::fs::{FileServer, relative};
use rocket_dyn_templates::Template;
use rocket_sync_db_pools::database;

embed_migrations!();

#[database("sqlite_database")]
pub struct DbConn(diesel::SqliteConnection);

async fn run_db_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    let conn = DbConn::get_one(&rocket).await.expect("database connection");
    // TODO: Do foreign keys work?
    match conn.run(|c| embedded_migrations::run(c)).await {
        Ok(()) => match conn.run(|c| c.execute("PRAGMA foreign_keys = ON")).await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to enable foreign keys: {:?}", e);
            }
        },
        Err(e) => {
            error!("Failed to run database migrations: {:?}", e);
        }
    }
    rocket
}

fn rocket() -> Rocket<Build> {
    rocket::build()
        .attach(DbConn::fairing())
        .attach(AdHoc::on_ignite("Database Migrations", run_db_migrations))
        .mount("/", FileServer::from(relative!("static")))
        .mount("/", routes![
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
        ])
        .attach(Template::fairing())
}

fn main() {
    rocket().launch();
}
