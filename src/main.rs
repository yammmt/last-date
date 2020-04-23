#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate rocket_contrib;

mod task;
// #[(cfg)test] mod tests;

use rocket::Rocket;
use rocket::fairing::AdHoc;
use rocket::request::{Form, FlashMessage};
// use rocket::response::{Flash, Redirect};
use rocket_contrib::{templates::Template, serve::StaticFiles};
use diesel::SqliteConnection;

use task::{Task, TaskName};

embed_migrations!();

#[database("sqlite_database")]
pub struct DbConn(SqliteConnection);

#[derive(Debug, Serialize)]
struct Context<'a, 'b>{ msg: Option<(&'a str, &'b str)>, tasks: Vec<Task> }

impl<'a, 'b> Context<'a, 'b> {
    pub fn err(conn: &DbConn, msg: &'a str) -> Context<'static, 'a> {
        Context{ msg: Some(("error", msg)), tasks: Task::all(conn) }
    }

    pub fn raw(conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> Context<'a, 'b> {
        Context{ msg: msg, tasks: Task::all(conn) }
    }
}

// TODO: add `update` and `delete`

#[post("/", data = "<task_form>")]
fn new(task_form: Form<TaskName>, conn: DbConn) -> Template {
    let task = task_form.into_inner();
    if task.name.is_empty() {
        Template::render("index", &Context::err(&conn, "Please input task name."))
    } else if Task::insert(task, &conn) {
        Template::render("index", Context::raw(&conn, None))
    } else {
        Template::render("index", &Context::err(&conn, "The server failed."))
    }
}

#[get("/")]
fn index(msg: Option<FlashMessage>, conn: DbConn) -> Template {
    Template::render("index", &match msg {
        Some(ref msg) => Context::raw(&conn, Some((msg.name(), msg.msg()))),
        None => Context::raw(&conn, None),
    })
}

fn run_db_migrations(rocket: Rocket)  -> Result<Rocket, Rocket> {
    let conn = DbConn::get_one(&rocket).expect("database connection");
    match embedded_migrations::run(&*conn) {
        Ok(()) => Ok(rocket),
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
        .mount("/", routes![index, new])
        .attach(Template::fairing())
}

fn main() {
    rocket().launch();
}
