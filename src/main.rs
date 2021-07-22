#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

mod models;
mod routes;
#[cfg(test)] mod tests;

use rocket::Rocket;
use rocket::fairing::AdHoc;
use rocket_contrib::{templates::Template, serve::StaticFiles};
use rocket_sync_db_pools::database;

embed_migrations!();

#[database("sqlite_database")]
pub struct DbConn(diesel::SqliteConnection);

fn run_db_migrations(rocket: Rocket)  -> Result<Rocket, Rocket> {
    let conn = DbConn::get_one(&rocket).expect("database connection");
    // TODO: Do foreign keys work?
    match embedded_migrations::run(&*conn) {
        Ok(()) =>  {
            match conn.execute("PRAGMA foreign_keys = ON") {
                Ok(_) => Ok(rocket),
                Err(e) => {
                    error!("Failed to enable foreign keys: {:?}", e);
                    Err(rocket)
                }
            }
        },
        Err(e) => {
            error!("Failed to run database migrations: {:?}", e);
            Err(rocket)
        }
    }
}

fn rocket() -> Rocket {
    rocket::ignite()
        .attach(DbConn::fairing())
        .attach(AdHoc::on_attach("Database Migrations", run_db_migrations))
        .mount("/", StaticFiles::from("static/"))
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
