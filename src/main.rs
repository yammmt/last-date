#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate rocket_contrib;

mod task;
#[cfg(test)] mod tests;

use rocket::Rocket;
use rocket::fairing::AdHoc;
use rocket::request::{Form, FlashMessage};
use rocket::response::{Flash, Redirect};
use rocket_contrib::{templates::Template, serve::StaticFiles};
use diesel::SqliteConnection;

use task::{Task, TaskName, TaskUpdate};

embed_migrations!();

#[database("sqlite_database")]
pub struct DbConn(SqliteConnection);

#[derive(Debug, Serialize)]
struct Context<'a, 'b>{ msg: Option<(&'a str, &'b str)>, tasks: Vec<Task> }
#[derive(Debug, Serialize)]
struct SingleTaskContext<'a, 'b>{ msg: Option<(&'a str, &'b str)>, task: Task }

impl<'a, 'b> Context<'a, 'b> {
    pub fn err(conn: &DbConn, msg: &'a str) -> Context<'static, 'a> {
        Context{ msg: Some(("warning", msg)), tasks: Task::all(conn) }
    }

    pub fn raw(conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> Context<'a, 'b> {
        Context{ msg, tasks: Task::all(conn) }
    }

}

impl<'a, 'b> SingleTaskContext<'a, 'b> {
    pub fn raw(id: i32, conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> SingleTaskContext<'a, 'b> {
        SingleTaskContext{ msg, task: Task::task_by_id(id, conn) }
    }
}

// TODO: add `delete`

#[post("/", data = "<task_form>")]
fn new(task_form: Form<TaskName>, conn: DbConn) -> Flash<Redirect> {
    let task = task_form.into_inner();
    if task.name.is_empty() {
        Flash::warning(Redirect::to("/"), "Please input task name.")
    } else if Task::insert(task, &conn) {
        Flash::success(Redirect::to("/"), "New task added.")
    } else {
        Flash::warning(Redirect::to("/"), "The server failed.")
    }
}

#[get("/")]
fn index(msg: Option<FlashMessage>, conn: DbConn) -> Template {
    Template::render("index", &match msg {
        Some(ref msg) => Context::raw(&conn, Some((msg.name(), msg.msg()))),
        None => Context::raw(&conn, None),
    })
}

#[post("/<id>/date")]
fn update_date(id: i32, conn: DbConn) -> Flash<Redirect> {
    if Task::update_to_today(id, &conn) {
        Flash::success(Redirect::to("/"), "\"Last updated\" date is updated to today.")
    } else {
        Flash::warning(Redirect::to("/"), "The server failed.")
    }
}

#[get("/<id>")]
fn task_detail(id: i32, msg: Option<FlashMessage>, conn: DbConn) -> Template {
    Template::render("detail", &match msg {
        Some(ref msg) => SingleTaskContext::raw(id, &conn, Some((msg.name(), msg.msg()))),
        None => SingleTaskContext::raw(id, &conn, None),
    })
}

#[post("/<id>", data = "<task_update_form>")]
fn update(id: i32, task_update_form: Form<TaskUpdate>, conn: DbConn) -> Flash<Redirect> {
    let task = task_update_form.into_inner();
    let redirect_url = format!("/{}", id);
    if task.name.is_empty() {
        Flash::warning(Redirect::to(redirect_url), "Please input task name.")
    } else if Task::update(id, task, &conn) {
        Flash::success(Redirect::to(redirect_url), "Your task was updated.")
    } else {
        Flash::warning(Redirect::to(redirect_url), "The server failed.")
    }
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
        .mount("/", routes![index, new, update_date, update, task_detail])
        .attach(Template::fairing())
}

fn main() {
    rocket().launch();
}
