extern crate rocket;
extern crate rocket_contrib;
extern crate serde_derive;

use crate::DbConn;
use crate::models::label::Label;
use crate::models::task::{Task, TaskName, TaskUpdate};

use rocket::request::{Form, FlashMessage};
use rocket::response::{Flash, Redirect};
use rocket_contrib::templates::Template;

#[derive(Debug, Serialize)]
struct Context<'a, 'b>{ msg: Option<(&'a str, &'b str)>, tasks: Vec<Task>, labels: Vec<Label> }
#[derive(Debug, Serialize)]
struct SingleTaskContext<'a, 'b>{ msg: Option<(&'a str, &'b str)>, task: Task, labels: Vec<Label> }
#[derive(Debug, Serialize)]
struct TasksByLabelContext{ tasks: Vec<Task>, label: Label}

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

impl TasksByLabelContext {
    pub fn raw(label_id: i32, conn: &DbConn) -> TasksByLabelContext {
        TasksByLabelContext{tasks: Task::tasks_by_label(label_id, conn), label: Label::label_by_id(label_id, conn)}
    }
}

#[post("/", data = "<task_form>")]
pub fn new(task_form: Form<TaskName>, conn: DbConn) -> Flash<Redirect> {
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
pub fn index(msg: Option<FlashMessage>, conn: DbConn) -> Template {
    Template::render("index", &match msg {
        Some(ref msg) => Context::raw(&conn, Some((msg.name(), msg.msg()))),
        None => Context::raw(&conn, None),
    })
}

#[get("/label/<id>", rank = 0)]
pub fn tasks_by_label(id: i32, conn: DbConn) -> Template {
    Template::render("tasksbylabel", TasksByLabelContext::raw(id, &conn))
}

#[post("/<id>/date", rank = 1)]
pub fn update_date(id: i32, conn: DbConn) -> Flash<Redirect> {
    if Task::update_to_today(id, &conn) {
        Flash::success(Redirect::to("/"), "\"Last updated\" date is updated to today.")
    } else {
        Flash::warning(Redirect::to("/"), "The server failed.")
    }
}

#[get("/<id>")]
pub fn task_detail(id: i32, msg: Option<FlashMessage>, conn: DbConn) -> Template {
    Template::render("detail", &match msg {
        Some(ref msg) => SingleTaskContext::raw(id, &conn, Some((msg.name(), msg.msg()))),
        None => SingleTaskContext::raw(id, &conn, None),
    })
}

#[post("/<id>", data = "<task_update_form>")]
pub fn update(id: i32, task_update_form: Form<TaskUpdate>, conn: DbConn) -> Flash<Redirect> {
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
pub fn confirm(id: i32, conn: DbConn) -> Template {
    Template::render("confirm", SingleTaskContext::raw(id, &conn, None))
}

#[delete("/<id>")]
pub fn delete(id: i32, conn: DbConn) -> Result<Flash<Redirect>, Template> {
    if Task::delete_with_id(id, &conn) {
        Ok(Flash::success(Redirect::to("/"), "Your task was deleted."))
    } else {
        Err(Template::render("detail", &Context::err(&conn, "Couldn't delete task.")))
    }
}
