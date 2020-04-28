// If we run tests, current all DB contents will be deleted.
// If you won't, please set environment variable for example:
// `export ROCKET_DATABASES='{sqlite_database={url="db/mydb.sqlite"}}'`

extern crate parking_lot;
extern crate rand;

use super::task::Task;
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
        assert!(body.contains("name"));
        assert!(body.contains("Last updated"));
        assert!(body.contains("Update to today"));
        // TODO: Ensure the number of table row reflects the number of tasks.
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
        let inserted_id = Task::id_by_name(&task_name, &conn);

        // Ensure we can access detail page.
        let mut res = client.get(format!("/{}", inserted_id)).dispatch();
        assert_eq!(res.status(), Status::Ok);

        // Ensure detail page shows required fields.
        let body = res.body_string().unwrap();
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
fn confirm_page() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "confirmpagetest".to_string();
        client.post("/")
            .header(ContentType::Form)
            .body(format!("name={}", task_name))
            .dispatch();
        let inserted_id = Task::id_by_name(&task_name, &conn);

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
fn test_many_insertions() {
    const ITER: usize = 100;

    let mut rng = thread_rng();
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
fn test_bad_update_form_submissions() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "detailformtest".to_string();
        client.post("/")
            .header(ContentType::Form)
            .body(format!("name={}", task_name))
            .dispatch();
        let inserted_id = Task::id_by_name(&task_name, &conn);
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
        let mut rng = thread_rng();
        let rng_name: String = rng.sample_iter(&Alphanumeric).take(7).collect();
        let t = Task::insert_with_old_date(&rng_name, &conn);
        assert!(t);

        // Ensure `updated_at` of created task is updated to today.
        let inserted_id = Task::id_by_name(&rng_name, &conn);
        let res = client.post(format!("/{}/date", inserted_id.to_string())).dispatch();
        let mut cookies = res.headers().get("Set-Cookie");
        let updated_date  = Task::updated_at_by_id(inserted_id, &conn);
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));
        assert_eq!(updated_date, Local::today().naive_local().to_string());
    })
}

#[test]
fn test_update_task() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name = "updatetasktest".to_string();
        let t = Task::insert_with_old_date(&task_name, &conn);
        assert!(t);

        // Submit valid update form. Note that `updated_at` field isn't updated.
        let inserted_id = Task::id_by_name(&task_name, &conn);
        let task_description = "newdescription".to_string();
        let dt = Local::today().naive_local().to_string();
        let form_data = format!("name={}&description={}&updated_at={}", task_name, task_description, dt);
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
    })
}
