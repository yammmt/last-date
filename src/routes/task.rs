extern crate rocket;
extern crate serde_derive;

use crate::DbConn;
use crate::models::label::Label;
use crate::models::task::{Task, TaskName, TaskUpdate};

use rocket::form::Form;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket_dyn_templates::Template;

#[derive(Debug, Serialize)]
struct IndexContext<'a, 'b>{ msg: Option<(&'a str, &'b str)>, tasks: Vec<Task>, labels: Vec<Label> }
#[derive(Debug, Serialize)]
struct SingleContext<'a, 'b>{ msg: Option<(&'a str, &'b str)>, task: Task, labels: Vec<Label> }
#[derive(Debug, Serialize)]
struct ByLabelContext{ tasks: Vec<Task>, label: Label}

impl<'a, 'b> IndexContext<'a, 'b> {
    pub async fn err(conn: &DbConn, msg: &'a str) -> IndexContext<'static, 'a> {
        IndexContext {
            msg: Some(("warning", msg)),
            tasks: Task::all(conn).await,
            labels: Label::all(conn).await,
        }
    }

    pub async fn raw(conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> IndexContext<'a, 'b> {
        IndexContext {
            msg,
            tasks: Task::all(conn).await,
            labels: Label::all(conn).await,
        }
    }
}

impl<'a, 'b> SingleContext<'a, 'b> {
    pub async fn raw(
        id: i32,
        conn: &DbConn,
        msg: Option<(&'a str, &'b str)>,
    ) -> SingleContext<'a, 'b> {
        SingleContext {
            msg,
            task: Task::task_by_id(id, conn).await,
            labels: Label::all(conn).await,
        }
    }
}

impl ByLabelContext {
    pub async fn raw(label_id: i32, conn: &DbConn) -> ByLabelContext {
        ByLabelContext {
            tasks: Task::tasks_by_label(label_id, conn).await,
            label: Label::label_by_id(label_id, conn).await,
        }
    }
}

#[post("/", data = "<task_form>")]
pub async fn new(task_form: Form<TaskName>, conn: DbConn) -> Flash<Redirect> {
    let task = task_form.into_inner();
    if task.name.is_empty() {
        Flash::warning(Redirect::to("/"), "Please input task name.")
    } else if Task::insert(task, &conn).await {
        Flash::success(Redirect::to("/"), "New task added.")
    } else {
        Flash::warning(Redirect::to("/"), "The server failed.")
    }
}

#[get("/")]
pub async fn index(msg: Option<FlashMessage<'_>>, conn: DbConn) -> Template {
    Template::render(
        "task/index",
        &match msg {
            Some(ref msg) => IndexContext::raw(&conn, Some((msg.kind(), msg.message()))).await,
            None => IndexContext::raw(&conn, None).await,
        },
    )
}

#[get("/label/<id>", rank = 0)]
pub async fn by_label(id: i32, conn: DbConn) -> Template {
    Template::render("task/bylabel", ByLabelContext::raw(id, &conn).await)
}

#[post("/<id>/date", rank = 1)]
pub async fn update_date(id: i32, conn: DbConn) -> Flash<Redirect> {
    if Task::update_to_today(id, &conn).await {
        Flash::success(Redirect::to("/"), "\"Last updated\" date is updated to today.")
    } else {
        Flash::warning(Redirect::to("/"), "The server failed.")
    }
}

#[get("/<id>")]
pub async fn edit(id: i32, msg: Option<FlashMessage<'_>>, conn: DbConn) -> Template {
    Template::render(
        "task/edit",
        &match msg {
            Some(ref msg) => SingleContext::raw(id, &conn, Some((msg.kind(), msg.message()))).await,
            None => SingleContext::raw(id, &conn, None).await,
        },
    )
}

#[post("/<id>", data = "<task_update_form>")]
pub async fn update(id: i32, task_update_form: Form<TaskUpdate>, conn: DbConn) -> Flash<Redirect> {
    let task = task_update_form.into_inner();
    let redirect_url = format!("/{}", id);
    if task.name.is_empty() {
        Flash::warning(Redirect::to(redirect_url), "Please input task name.")
    } else if Task::update(id, task, &conn).await {
        Flash::success(Redirect::to(redirect_url), "Your task was updated.")
    } else {
        Flash::warning(Redirect::to(redirect_url), "The server failed.")
    }
}

#[get("/<id>/confirm", rank = 1)]
pub async fn confirm(id: i32, conn: DbConn) -> Template {
    Template::render("task/confirm", SingleContext::raw(id, &conn, None).await)
}

#[delete("/<id>")]
pub async fn delete(id: i32, conn: DbConn) -> Result<Flash<Redirect>, Template> {
    if Task::delete_with_id(id, &conn).await {
        Ok(Flash::success(Redirect::to("/"), "Your task was deleted."))
    } else {
        Err(Template::render(
            "task/index",
            &IndexContext::err(&conn, "Couldn't delete task.").await,
        ))
    }
}
