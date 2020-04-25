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

            // Record the description we choose for this iteration.
            descs.insert(0, desc);

            // Ensure the task was inserted properly and all other tasks remain.
            let tasks = Task::all(&conn);
            assert_eq!(tasks.len(), init_num + i + 1);

            for j in 0..i {
                assert_eq!(descs[j], tasks[j].name)
            }
        }
    })
}

#[test]
fn test_bad_form_submissions() {
    run_test!(|client, _conn| {
        // Submit an **empty** form. This is an unexpected pattern
        // because task form in index page has `name` field.
        let res = client.post("/")
            .header(ContentType::Form)
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        println!("{:?}", res.status());
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a name field. This is same as just above pattern.
        let res = client.post("/")
            .header(ContentType::Form)
            .dispatch();

        let mut cookies = res.headers().get("Set-Cookie");
        println!("{:?}", res.status());
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
