extern crate regex;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde_derive;

use crate::DbConn;
use crate::models::label::{Label, LabelForm};

use regex::Regex;
use rocket::request::{Form, FlashMessage};
use rocket::response::{Flash, Redirect};
use rocket_contrib::templates::Template;

#[derive(Debug, Serialize)]
struct Context<'a, 'b>{ msg: Option<(&'a str, &'b str)>, labels: Vec<Label> }
#[derive(Debug, Serialize)]
struct SingleLabelContext{ label: Label }
#[derive(Debug, Serialize)]
struct LabelEditContext<'a, 'b>{ msg: Option<(&'a str, &'b str)>, label: Label }

impl<'a, 'b> Context<'a, 'b> {
    pub fn err(conn: &DbConn, msg: &'a str) -> Context<'static, 'a> {
        Context{ msg: Some(("warning", msg)), labels: Label::all(conn) }
    }

    pub fn raw(conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> Context<'a, 'b> {
        Context{ msg, labels: Label::all(conn) }
    }
}

impl SingleLabelContext {
    pub fn raw(id: i32, conn: &DbConn) -> SingleLabelContext {
        SingleLabelContext{ label: Label::label_by_id(id, conn) }
    }
}

impl<'a, 'b> LabelEditContext<'a, 'b> {
    pub fn raw(id: i32, conn: &DbConn, msg: Option<(&'a str, &'b str)>) -> LabelEditContext<'a, 'b> {
        LabelEditContext{ msg, label: Label::label_by_id(id, conn) }
    }
}

#[post("/label", data = "<label_form>")]
pub fn new_label(label_form: Form<LabelForm>, conn: DbConn) -> Flash<Redirect> {
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
pub fn label_list(msg: Option<FlashMessage>, conn: DbConn) -> Template {
    Template::render("labellist", &match msg {
        Some(ref msg) => Context::raw(&conn, Some((msg.name(), msg.msg()))),
        None => Context::raw(&conn, None),
    })
}

#[post("/label/<id>", data="<label_form>")]
pub fn label_update(id: i32, label_form: Form<LabelForm>, conn: DbConn) -> Flash<Redirect> {
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

#[get("/label/<id>/edit", rank = 0)]
pub fn label_edit(id: i32, msg: Option<FlashMessage>, conn: DbConn) -> Template {
    Template::render("labeledit", &match msg {
        Some(ref msg) => LabelEditContext::raw(id, &conn, Some((msg.name(), msg.msg()))),
        None => LabelEditContext::raw(id, &conn, None),
    })
}

#[get("/label/<id>/confirm")]
pub fn label_confirm(id: i32, conn: DbConn) -> Template {
    Template::render("labelconfirm", SingleLabelContext::raw(id, &conn))
}

#[delete("/label/<id>")]
pub fn label_delete(id: i32, conn: DbConn) -> Result<Flash<Redirect>, Template> {
    if Label::delete_with_id(id, &conn) {
        Ok(Flash::success(Redirect::to("/label"), "Your label was deleted."))
    } else {
        Err(Template::render("labeledit", &Context::err(&conn, "Couldn't delete label.")))
    }
}
