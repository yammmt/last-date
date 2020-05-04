#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate rocket_contrib;

extern crate regex;

mod label;
mod task;
#[cfg(test)] mod tests;

use regex::Regex;
use rocket::Rocket;
use rocket::fairing::AdHoc;
use rocket::request::{Form, FlashMessage};
use rocket::response::{Flash, Redirect};
use rocket_contrib::{templates::Template, serve::StaticFiles};
use diesel::SqliteConnection;
use diesel::Connection;

use label::{Label, LabelForm};
use task::{Task, TaskName, TaskUpdate};

embed_migrations!();

#[database("sqlite_database")]
pub struct DbConn(SqliteConnection);

#[derive(Debug, Serialize)]
struct Context<'a, 'b>{ msg: Option<(&'a str, &'b str)>, tasks: Vec<Task>, labels: Vec<Label> }
#[derive(Debug, Serialize)]
struct SingleTaskContext<'a, 'b>{ msg: Option<(&'a str, &'b str)>, task: Task, labels: Vec<Label> }
#[derive(Debug, Serialize)]
struct SingleLabelContext{ label: Label }
#[derive(Debug, Serialize)]
struct TasksByLabelContext{ tasks: Vec<Task>, label: Label}
#[derive(Debug, Serialize)]
struct LabelEditContext<'a, 'b>{ msg: Option<(&'a str, &'b str)>, label: Label }

impl<'a, 'b> Context<'a, 'b> {
    pub fn err(conn: &DbConn, msg: &'a str) -> Context<'static, 'a> {
        Context{ msg: Some(("warning", msg)), tasks: Task::all(conn), labels: Label::all(conn) }
    }

    pub fn raw(conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> Context<'a, 'b> {
        Context{ msg, tasks: Task::all(conn), labels: Label::all(conn) }
    }

}

impl<'a, 'b> SingleTaskContext<'a, 'b> {
    pub fn raw(id: i32, conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> SingleTaskContext<'a, 'b> {
        SingleTaskContext{ msg, task: Task::task_by_id(id, conn), labels: Label::all(conn) }
    }
}

impl SingleLabelContext {
    pub fn raw(id: i32, conn: &DbConn) -> SingleLabelContext {
        SingleLabelContext{ label: Label::label_by_id(id, conn) }
    }
}

impl TasksByLabelContext {
    pub fn raw(label_id: i32, conn: &DbConn) -> TasksByLabelContext {
        TasksByLabelContext{tasks: Task::tasks_by_label(label_id, conn), label: Label::label_by_id(label_id, conn)}
    }
}

impl<'a, 'b> LabelEditContext<'a, 'b> {
    pub fn raw(id: i32, conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> LabelEditContext<'a, 'b> {
        LabelEditContext{ msg, label: Label::label_by_id(id, conn) }
    }
}

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

#[post("/label", data = "<label_form>")]
fn new_label(label_form: Form<LabelForm>, conn: DbConn) -> Flash<Redirect> {
    let label = label_form.into_inner();
    let color_code_regex = Regex::new(r"#[[:xdigit:]]{6}$").unwrap();
    if label.name.is_empty() {
        Flash::warning(Redirect::to("/label"), "Please input label name.")
    } else if label.color.is_empty() || !color_code_regex.is_match(&label.color) {
        Flash::warning(Redirect::to("/label"), "Please input label color with hex format.")
    } else if Label::insert(label, &conn) {
        Flash::success(Redirect::to("/label"), "New label added.")
    } else {
        Flash::warning(Redirect::to("/label"), "The server failed.")
    }
}

#[get("/label")]
fn label_list(msg: Option<FlashMessage>, conn: DbConn) -> Template {
    Template::render("labellist", &match msg {
        Some(ref msg) => Context::raw(&conn, Some((msg.name(), msg.msg()))),
        None => Context::raw(&conn, None),
    })
}

#[post("/label/<id>", data="<label_form>")]
fn label_update(id: i32, label_form: Form<LabelForm>, conn: DbConn) -> Flash<Redirect> {
    let label = label_form.into_inner();
    let color_code_regex = Regex::new(r"#[[:xdigit:]]{6}$").unwrap();
    let redirect_url = format!("/label/{}/edit", id);
    if label.name.is_empty() {
        Flash::warning(Redirect::to(redirect_url), "Please input label name.")
    } else if label.color.is_empty() || !color_code_regex.is_match(&label.color) {
        Flash::warning(Redirect::to(redirect_url), "Please input label color with hex format.")
    } else if Label::update(id, label, &conn) {
        Flash::success(Redirect::to(redirect_url), "Label is updated.")
    } else {
        Flash::warning(Redirect::to(redirect_url), "The server failed.")
    }
}

#[get("/label/<id>", rank = 0)]
fn tasks_by_label(id: i32, conn: DbConn) -> Template {
    Template::render("tasksbylabel", TasksByLabelContext::raw(id, &conn))
}

#[get("/label/<id>/edit", rank = 0)]
fn label_edit(id: i32, msg: Option<FlashMessage>, conn: DbConn) -> Template {
    Template::render("labeledit", &match msg {
        Some(ref msg) => LabelEditContext::raw(id, &conn, Some((msg.name(), msg.msg()))),
        None => LabelEditContext::raw(id, &conn, None),
    })
}

#[get("/label/<id>/confirm")]
fn label_confirm(id: i32, conn: DbConn) -> Template {
    Template::render("labelconfirm", SingleLabelContext::raw(id, &conn))
}

#[delete("/label/<id>")]
fn label_delete(id: i32, conn: DbConn) -> Result<Flash<Redirect>, Template> {
    if Label::delete_with_id(id, &conn) {
        Ok(Flash::success(Redirect::to("/label"), "Your label was deleted."))
    } else {
        Err(Template::render("labeledit", &Context::err(&conn, "Couldn't delete label.")))
    }
}

#[post("/<id>/date", rank = 1)]
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

#[get("/<id>/confirm", rank = 1)]
fn confirm(id: i32, conn: DbConn) -> Template {
    Template::render("confirm", SingleTaskContext::raw(id, &conn, None))
}

#[delete("/<id>")]
fn delete(id: i32, conn: DbConn) -> Result<Flash<Redirect>, Template> {
    if Task::delete_with_id(id, &conn) {
        Ok(Flash::success(Redirect::to("/"), "Your task was deleted."))
    } else {
        Err(Template::render("detail", &Context::err(&conn, "Couldn't delete task.")))
    }
}

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
        .mount("/", routes![index, new, update_date, update, task_detail, delete, confirm, label_list, new_label, tasks_by_label, label_edit, label_update, label_confirm, label_delete])
        .attach(Template::fairing())
}

fn main() {
    rocket().launch();
}
