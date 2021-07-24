use crate::DbConn;
use crate::models::label::{Label, LabelForm};

use regex::Regex;
use rocket::form::Form;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::serde::Serialize;
use rocket_dyn_templates::Template;

#[derive(Debug, Serialize)]
struct IndexContext<'a, 'b>{ msg: Option<(&'a str, &'b str)>, labels: Vec<Label> }
#[derive(Debug, Serialize)]
struct SingleContext{ label: Label }
#[derive(Debug, Serialize)]
struct UpdateContext<'a, 'b>{ msg: Option<(&'a str, &'b str)>, label: Label }

impl<'a, 'b> IndexContext<'a, 'b> {
    pub async fn err(conn: &DbConn, msg: &'a str) -> IndexContext<'static, 'a> {
        IndexContext {
            msg: Some(("warning", msg)),
            labels: Label::all(conn).await,
        }
    }

    pub async fn raw(conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> IndexContext<'a, 'b> {
        IndexContext {
            msg,
            labels: Label::all(conn).await,
        }
    }
}

impl SingleContext {
    pub async fn raw(id: i32, conn: &DbConn) -> SingleContext {
        SingleContext {
            label: Label::label_by_id(id, conn).await,
        }
    }
}

impl<'a, 'b> UpdateContext<'a, 'b> {
    pub async fn raw(
        id: i32,
        conn: &DbConn,
        msg: Option<(&'a str, &'b str)>,
    ) -> UpdateContext<'a, 'b> {
        UpdateContext {
            msg,
            label: Label::label_by_id(id, conn).await,
        }
    }
}

#[post("/label", data = "<label_form>")]
pub async fn new(label_form: Form<LabelForm>, conn: DbConn) -> Flash<Redirect> {
    let label = label_form.into_inner();
    let color_code_regex = Regex::new(r"#[[:xdigit:]]{6}$").unwrap();
    if label.name.is_empty() {
        Flash::warning(Redirect::to("/label"), "Please input label name.")
    } else if label.color.is_empty() || !color_code_regex.is_match(&label.color) {
        Flash::warning(Redirect::to("/label"), "Please input label color with hex format.")
    } else if Label::insert(label, &conn).await {
        Flash::success(Redirect::to("/label"), "New label added.")
    } else {
        Flash::warning(Redirect::to("/label"), "The server failed.")
    }
}

#[get("/label")]
pub async fn index(msg: Option<FlashMessage<'_>>, conn: DbConn) -> Template {
    Template::render(
        "label/index",
        &match msg {
            Some(ref msg) => IndexContext::raw(&conn, Some((msg.kind(), msg.message()))).await,
            None => IndexContext::raw(&conn, None).await,
        },
    )
}

#[post("/label/<id>", data="<label_form>")]
pub async fn update(id: i32, label_form: Form<LabelForm>, conn: DbConn) -> Flash<Redirect> {
    let label = label_form.into_inner();
    let color_code_regex = Regex::new(r"#[[:xdigit:]]{6}$").unwrap();
    let redirect_url = format!("/label/{}/edit", id);
    if label.name.is_empty() {
        Flash::warning(Redirect::to(redirect_url), "Please input label name.")
    } else if label.color.is_empty() || !color_code_regex.is_match(&label.color) {
        Flash::warning(Redirect::to(redirect_url), "Please input label color with hex format.")
    } else if Label::update(id, label, &conn).await {
        Flash::success(Redirect::to(redirect_url), "Label is updated.")
    } else {
        Flash::warning(Redirect::to(redirect_url), "The server failed.")
    }
}

#[get("/label/<id>/edit", rank = 0)]
pub async fn edit(id: i32, msg: Option<FlashMessage<'_>>, conn: DbConn) -> Template {
    Template::render(
        "label/edit",
        &match msg {
            Some(ref msg) => UpdateContext::raw(id, &conn, Some((msg.kind(), msg.message()))).await,
            None => UpdateContext::raw(id, &conn, None).await,
        },
    )
}

#[get("/label/<id>/confirm")]
pub async fn confirm(id: i32, conn: DbConn) -> Template {
    Template::render("label/confirm", SingleContext::raw(id, &conn).await)
}

#[delete("/label/<id>")]
pub async fn delete(id: i32, conn: DbConn) -> Result<Flash<Redirect>, Template> {
    if Label::delete_with_id(id, &conn).await {
        Ok(Flash::success(Redirect::to("/label"), "Your label was deleted."))
    } else {
        Err(Template::render(
            "label/index",
            &IndexContext::err(&conn, "Couldn't delete label.").await,
        ))
    }
}
