// If we run tests, current all DB contents will be deleted.
// If you won't, please set environment variable for example:
// `export ROCKET_DATABASES='{sqlite_database={url="db/mydb.sqlite"}}'`

extern crate parking_lot;
extern crate rand;

use super::models::label::Label;
use super::models::task::Task;
use self::parking_lot::Mutex;
use self::rand::{Rng, thread_rng, distributions::Alphanumeric};

use rocket::local::Client;
use rocket::http::{Status, ContentType};
use chrono::Local;

static DB_LOCK: Mutex<()> = Mutex::new(());

macro_rules! run_test {
    (|$client:ident, $conn:ident| $block:expr) => ({
        let _lock = DB_LOCK.lock();
        let rocket = super::rocket();
        let db = super::DbConn::get_one(&rocket);
        let $client = Client::new(rocket).expect("Rocket client");
        let $conn = db.expect("failed to get database connection for testing");
        assert!(Task::delete_all(&$conn), "failed to delete all tasks for testing");
        assert!(Label::delete_all(&$conn), "failed to delete all labels for testing");

        $block
    })
}

#[test]
fn index_page() {
    run_test!(|client, _conn| {
        // Ensure we can access index page
        let mut res = client.get("/").dispatch();
        assert_eq!(res.status(), Status::Ok);

        // Ensure index shows correct task table.
        let body = res.body_string().unwrap();
        assert!(body.contains("Label"));
        assert!(body.contains("name"));
        assert!(body.contains("Last updated"));
        assert!(body.contains("Update to today"));
        // TODO: Ensure the number of table row reflects the number of tasks.
    })
}

#[test]
fn label_list_page() {
    run_test!(|client, conn| {
        // TODO: use rand for hex color code, too
        let rng = thread_rng();
        let name: String = rng.sample_iter(&Alphanumeric).take(6).collect();

        // Ensure we can access label list page
        let mut res = client.get("/label").dispatch();
        assert_eq!(res.status(), Status::Ok);
        let body = res.body_string().unwrap();
        assert!(!body.contains(&name));

        // Ensure created label is shown in label list page
        client.post("/label")
            .header(ContentType::Form)
            .body(format!("name={}&color=#ababab", name))
            .dispatch();

        let mut res = client.get("/label").dispatch();
        assert_eq!(res.status(), Status::Ok);
        let body = res.body_string().unwrap();
        assert!(body.contains(&name));
    })
}

#[test]
fn detail_page() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "detailpagetest".to_string();
        client.post("/")
            .header(ContentType::Form)
            .body(format!("name={}", task_name))
            .dispatch();
        let inserted_id = Task::all(&conn)[0].id.unwrap();

        // Ensure we can access detail page.
        let mut res = client.get(format!("/{}", inserted_id)).dispatch();
        assert_eq!(res.status(), Status::Ok);

        // Ensure detail page shows required fields.
        let body = res.body_string().unwrap();
        assert!(body.contains("Label"));
        assert!(body.contains("Task name"));
        assert!(body.contains("Description"));
        assert!(body.contains("Last updated"));
        assert!(body.contains(r#"<button class="button is-primary is-light" type="submit">Update</button>"#));
        assert!(body.contains(
            r#"<button class="button is-link is-light" onclick="location.href='../'">Back to index page</button>"#
        ));
    })
}

#[test]
fn tasks_by_label_page() {
    let rng = thread_rng();
    run_test!(|client, conn| {
        // Create new tasks
        let mut task_names: Vec<String> = Vec::with_capacity(3);
        let mut task_ids: [i32; 3] = [0, 0, 0];
        for i in 0..3 {
            let rng_name: String = rng.sample_iter(&Alphanumeric).take(7).collect();
            client.post("/")
                .header(ContentType::Form)
                .body(format!("name={}", rng_name))
                .dispatch();
            let inserted_id = Task::all(&conn)[i].id.unwrap();
            task_names.push(rng_name);
            task_ids[i] = inserted_id;
        }

        // Create a new label, too.
        client.post("/label")
            .header(ContentType::Form)
            .body(format!("name=newlabel&color=#eeeeee"))
            .dispatch();
        let inserted_label_id = Label::all(&conn)[0].id.unwrap();

        // Attach label to several tasks.
        let dt = Local::today().naive_local().to_string();
        for i in 0..2 {
            let form_data = format!("name={}&description=&updated_at={}&label_id={}", &task_names[i], dt, inserted_label_id);
            client.post(format!("/{}", task_ids[i]))
                .header(ContentType::Form)
                .body(form_data)
                .dispatch();
        }

        // Ensure several tasks are shown.
        let mut res = client.get(format!("/label/{}", inserted_label_id)).dispatch();
        assert_eq!(res.status(), Status::Ok);

        let body = res.body_string().unwrap();
        assert!(body.contains(&task_names[0]));
        assert!(body.contains(&task_names[1]));
        assert!(!body.contains(&task_names[2]));
    })
}

#[test]
fn confirm_page() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "confirmpagetest".to_string();
        client.post("/")
            .header(ContentType::Form)
            .body(format!("name={}", task_name))
            .dispatch();
        let inserted_id = Task::all(&conn)[0].id.unwrap();

        // Ensure we can access detail page.
        let mut res = client.get(format!("/{}/confirm", inserted_id)).dispatch();
        assert_eq!(res.status(), Status::Ok);

        // Ensure confirm page shows buttons
        let body = res.body_string().unwrap();
        assert!(body.contains(
            r#"<button class="button is-danger is-light" type="submit">Delete</button>"#
        ));
        // If I write full button HTML, `cargo test` hangs up. I don't know why.
        assert!(body.contains("Back to task</button>"));
        assert!(body.contains(
            r#"<button class="button is-link is-light" onclick="location.href='/'">Back to index page</button>"#
        ));
    })
}

#[test]
fn label_confirm_page() {
    run_test!(|client, conn| {
        // Create a new label.
        client.post("/label")
            .header(ContentType::Form)
            .body("name=label+confirm+test&color=#ababab")
            .dispatch();
            let inserted_id = Label::all(&conn)[0].id.unwrap();

        // Ensure we can access confirm page.
        let mut res = client.get(format!("/label/{}/confirm", inserted_id)).dispatch();
        assert_eq!(res.status(), Status::Ok);

        // Ensure confirm page shows buttons
        let body = res.body_string().unwrap();
        assert!(body.contains(
            r#"<button class="button is-danger is-light" type="submit">Delete</button>"#
        ));
        assert!(body.contains("Back to label</button>"));
        assert!(body.contains(
            r#"<button class="button is-link is-light" onclick="location.href='/'">Back to index page</button>"#
        ));
    })
}

#[test]
fn test_insertion_deletion() {
    run_test!(|client, conn| {
        // Get the tasks before making changes.
        let init_tasks = Task::all(&conn);

        // insert new task
        client.post("/")
            .header(ContentType::Form)
            .body("name=test+task")
            .dispatch();

        // Ensure we have one more task in the DB.
        let new_tasks = Task::all(&conn);
        assert_eq!(new_tasks.len(), init_tasks.len() + 1);

        // Ensure the task is what we expect.
        assert_eq!(new_tasks[0].name, "test task");
        assert_eq!(new_tasks[0].description, "");
        assert_eq!(new_tasks[0].updated_at, Local::today().naive_local().to_string());
        assert_eq!(new_tasks[0].label_id, None);

        // Delete task.
        let id = new_tasks[0].id.unwrap();
        client.delete(format!("/{}", id)).dispatch();

        // Ensure task was deleted.
        let final_tasks = Task::all(&conn);
        assert_eq!(final_tasks.len(), init_tasks.len());
        if final_tasks.len() > 0 {
            assert_ne!(final_tasks[0].name, "test task");
        }
    })
}

#[test]
fn test_label_insertion_deletion() {
    run_test!(|client, conn| {
        // Get the labels before making changes.
        let init_labels = Label::all(&conn);

        // Insert new label.
        client.post("/label")
            .header(ContentType::Form)
            .body("name=test+label&color=#ababab")
            .dispatch();

        // Ensure we have one more label in the DB.
        let new_labels = Label::all(&conn);
        assert_eq!(new_labels.len(), init_labels.len() + 1);

        // Ensure the label is what we expect.
        assert_eq!(new_labels[0].name, "test label");
        assert_eq!(new_labels[0].color_hex, "#ababab");

        // Delete a label.
        let id = new_labels[0].id.unwrap();
        client.delete(format!("/label/{}", id)).dispatch();

        // Ensure label was deleted.
        let final_labels = Label::all(&conn);
        assert_eq!(final_labels.len(), init_labels.len());
        if final_labels.len() > 0 {
            assert_ne!(final_labels[0].name, "test label");
        }
    })
}

#[test]
fn test_many_insertions() {
    const ITER: usize = 100;

    let rng = thread_rng();
    run_test!(|client, conn| {
        let init_num = Task::all(&conn).len();
        let mut descs = Vec::new();

        for i in 0..ITER {
            // Insert new task with random name.
            let desc: String = rng.sample_iter(&Alphanumeric).take(6).collect();
            client.post("/")
                .header(ContentType::Form)
                .body(format!("name={}", desc))
                .dispatch();

            // Record the name we choose for this iteration.
            descs.insert(0, desc);

            // Ensure the task was inserted properly and all other tasks remain.
            let tasks = Task::all_by_id(&conn);
            assert_eq!(tasks.len(), init_num + i + 1);

            for j in 0..i {
                assert_eq!(descs[j], tasks[j].name);
            }
        }
    })
}

#[test]
fn test_bad_new_task_form_submissions() {
    run_test!(|client, _conn| {
        // Submit an **empty** form. This is an unexpected pattern
        // because task form in index page has `name` field.
        let res = client.post("/")
            .header(ContentType::Form)
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a name field. This is same as just above pattern.
        let res = client.post("/")
            .header(ContentType::Form)
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form with an empty name. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client.post("/")
            .header(ContentType::Form)
            .body("name=")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));
    })
}

#[test]
fn test_bad_new_label_form_submissions() {
    run_test!(|client, _conn| {
        // Submit an **empty** form. This is an unexpected pattern
        // because label form in index page has `name` and `color` field.
        let res = client.post("/label")
            .header(ContentType::Form)
            .dispatch();

            let mut cookies = res.headers().get("Set-Cookie");
            assert_eq!(res.status(), Status::UnprocessableEntity);
            assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a name field.
        let res = client.post("/label")
            .header(ContentType::Form)
            .body("color=#123456")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a color field.
        let res = client.post("/label")
            .header(ContentType::Form)
            .body("name=mylabel")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form with an empty name. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client.post("/label")
            .header(ContentType::Form)
            .body("name=&color=#ff00ff")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));

        // Submit a form with an empty color. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client.post("/label")
            .header(ContentType::Form)
            .body("name=mylabel&color=")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));

        // Submit a form with an invalid color. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client.post("/label")
            .header(ContentType::Form)
            .body("name=mylabel&color=red")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));

        // Submit a form with invalid color (color code has 7 digits).
        let res = client.post("/label")
            .header(ContentType::Form)
            .body("name=mylabel&color=#1234567")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));

        // Submit a form with invalid color (color code has 5 digits).
        let res = client.post("/label")
            .header(ContentType::Form)
            .body("name=mylabel&color=#12345")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));
    })
}

#[test]
fn test_bad_update_form_submissions() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "detailformtest".to_string();
        client.post("/")
            .header(ContentType::Form)
            .body(format!("name={}", task_name))
            .dispatch();
        let inserted_id = Task::all(&conn)[0].id.unwrap();
        let post_url = format!("/{}", inserted_id);

        // Submit an **empty** form. This is an unexpected pattern
        // because task form in detail page has some fields.
        let res = client.post(&post_url)
            .header(ContentType::Form)
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a name field. This is same as just above pattern.
        let res = client.post(&post_url)
            .header(ContentType::Form)
            .body("description=hello")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form with an empty name. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client.post(&post_url)
            .header(ContentType::Form)
            .body("name=&description=hello&updated_at=2020-04-28")
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));
    })
}

#[test]
fn test_update_date() {
    run_test!(|client, conn| {
        // Create new task with old `updated_at`.
        let rng = thread_rng();
        let rng_name: String = rng.sample_iter(&Alphanumeric).take(7).collect();
        let t = Task::insert_with_old_date(&rng_name, &conn);
        assert!(t);

        // Ensure `updated_at` of created task is updated to today.
        let new_tasks = Task::all(&conn);
        let today_str = Local::today().naive_local().to_string();
        // First, ensure current task date is not today.
        assert_ne!(new_tasks[0].updated_at, today_str);

        let inserted_id = new_tasks[0].id.unwrap(); // `id` is `Nullable`
        let res = client.post(format!("/{}/date", inserted_id)).dispatch();
        let mut cookies = res.headers().get("Set-Cookie");
        let final_tasks = Task::all(&conn);
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));
        assert_eq!(final_tasks[0].updated_at, today_str);
    })
}

#[test]
fn test_update_task() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name = "updatetasktest".to_string();
        let t = Task::insert_with_old_date(&task_name, &conn);
        assert!(t);

        // Create new label, too.
        client.post("/label")
            .header(ContentType::Form)
            .body(format!("name=newlabel&color=#eeeeee"))
            .dispatch();

        // Submit valid update form. Note that `updated_at` field isn't updated.
        let inserted_id = Task::all(&conn)[0].id.unwrap();
        let inserted_label_id = Label::all(&conn)[0].id.unwrap();
        let task_description = "newdescription".to_string();
        let dt = Local::today().naive_local().to_string();
        let form_data = format!("name={}&description={}&updated_at={}&label_id={}", task_name, task_description, dt, inserted_label_id);
        let res = client.post(format!("/{}", inserted_id))
            .header(ContentType::Form)
            .body(form_data)
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));

        let updated_task = Task::task_by_id(inserted_id, &conn);
        assert_eq!(updated_task.name, task_name);
        assert_eq!(updated_task.description, task_description);
        assert_ne!(updated_task.updated_at, dt);
        assert_eq!(updated_task.label_id, Some(inserted_label_id));

        // Update label_id to NULL.
        let form_data = format!("name={}&description={}&updated_at={}&label_id=", task_name, task_description, dt);
        let res = client.post(format!("/{}", inserted_id))
            .header(ContentType::Form)
            .body(form_data)
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));

        let updated_task = Task::task_by_id(inserted_id, &conn);
        assert_eq!(updated_task.label_id, None);
    })
}

#[test]
fn test_update_label() {
    run_test!(|client, conn| {
        // Create a new label.
        client.post("/label")
            .header(ContentType::Form)
            .body(format!("name=newlabel&color=#eeeeee"))
            .dispatch();
        let inserted_id = Label::all(&conn)[0].id.unwrap();

        // Update above label.
        let new_name = "newnewlabel".to_string();
        let new_color = "#5566ff".to_string();
        let form_data = format!("name={}&color={}", &new_name, &new_color);
        let res = client.post(format!("/label/{}", inserted_id))
            .header(ContentType::Form)
            .body(form_data)
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));

        let updated_label = Label::label_by_id(inserted_id, &conn);
        assert_eq!(updated_label.name, new_name);
        assert_eq!(updated_label.color_hex, new_color);
    })
}
