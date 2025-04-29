use super::models::label::Label;
use super::models::task::Task;

use parking_lot::{const_mutex, Mutex};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use dotenv;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use scraper::{Html, Selector};

static DB_LOCK: Mutex<()> = const_mutex(());

macro_rules! run_test {
    (|$client:ident, $conn:ident| $block:expr) => {{
        let _lock = DB_LOCK.lock();

        // Load environment variables from .env.test
        dotenv::from_filename(".env.test").ok();

        rocket::async_test(async move {
            let $client = Client::tracked(super::rocket())
                .await
                .expect("Rocket client");
            let db = super::DbConn::get_one(&$client.rocket()).await;
            let $conn = db.expect("failed to get database connection for testing");
            assert!(
                Task::delete_all(&$conn).await,
                "failed to delete all tasks for testing"
            );
            assert!(
                Label::delete_all(&$conn).await,
                "failed to delete all labels for testing"
            );

            $block
        })
    }};
}

#[test]
fn index_shows_main_task_table_headers_and_buttons() {
    run_test!(|client, _conn| {
        // Ensure we can access index page
        let res = client.get("/").dispatch().await;
        assert_eq!(res.status(), Status::Ok);

        // Ensure index shows correct task table.
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let header_selector = Selector::parse("th").unwrap();
        let headers: Vec<String> = document
            .select(&header_selector)
            .map(|el| el.text().collect::<String>().trim().to_string())
            .collect();
        // Check for each expected header
        assert!(
            headers.iter().any(|h| h.contains("Label")),
            "Task table header 'Label' could be missing"
        );
        assert!(
            headers.iter().any(|h| h == "Name"),
            "Task table header 'Name' missing"
        );
        assert!(
            headers.iter().any(|h| h == "Last updated"),
            "Task table header 'Last updated' missing"
        );
        assert!(
            headers.iter().any(|h| h == "Update to today"),
            "'Update to today' button missing"
        );
        // TODO: Ensure the number of table row reflects the number of tasks.
    })
}

#[test]
fn label_list_displays_created_and_absent_labels() {
    run_test!(|client, conn| {
        // TODO: use rand for hex color code, too
        let mut rng = thread_rng();
        let name: String = (&mut rng)
            .sample_iter(&Alphanumeric)
            .map(char::from)
            .take(6)
            .collect();

        // Ensure we can access label list page
        let res = client.get("/label").dispatch().await;
        assert_eq!(
            res.status(),
            Status::Ok,
            "Label list page returned non-200 status"
        );
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        // Check that label name is NOT present before creation
        let label_selector = Selector::parse("td,li,span").unwrap();
        let label_found = document
            .select(&label_selector)
            .any(|el| el.text().any(|t| t.trim() == name));
        assert!(
            !label_found,
            "Label name '{}' unexpectedly found in label list page",
            name
        );

        // Ensure created label is shown in label list page
        client
            .post("/label")
            .header(ContentType::Form)
            .body(format!("name={}&color=#ababab", name))
            .dispatch()
            .await;

        let res = client.get("/label").dispatch().await;
        assert_eq!(
            res.status(),
            Status::Ok,
            "Label list page returned non-200 status after label creation"
        );
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        // Check that label name IS present after creation
        let label_selector = Selector::parse("td,li,span").unwrap();
        let label_found = document
            .select(&label_selector)
            .any(|el| el.text().any(|t| t.trim() == name));
        assert!(
            label_found,
            "Label name '{}' not found in label list page after creation",
            name
        );
    })
}

#[test]
fn task_detail_page_displays_fields_and_buttons() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "detailpagetest".to_string();
        client
            .post("/")
            .header(ContentType::Form)
            .body(format!("name={}", task_name))
            .dispatch()
            .await;
        let inserted_id = Task::all(&conn).await[0].id.unwrap();

        // Ensure we can access detail page.
        let res = client.get(format!("/{}", inserted_id)).dispatch().await;
        assert_eq!(
            res.status(),
            Status::Ok,
            "Detail page returned non-200 status"
        );

        // Ensure detail page shows required fields using HTML parser.
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        // Check for field labels
        let label_texts = ["Label", "Task name", "Description", "Last updated"];
        for expected in &label_texts {
            assert!(
                body.contains(expected),
                "Detail page missing '{}' field",
                expected
            );
        }
        // Check for Update button
        let update_button = document
            .select(&Selector::parse("button[type='submit']").unwrap())
            .find(|el| el.text().any(|t| t.contains("Update")));
        assert!(
            update_button.is_some(),
            "Detail page missing 'Update' button"
        );
        // Check for Back button or link
        let back_button = document
            .select(&Selector::parse("button[onclick],a[onclick]").unwrap())
            .find(|el| {
                el.value()
                    .attr("onclick")
                    .map_or(false, |v| v.contains("location.href='../'"))
            });
        assert!(
            back_button.is_some(),
            "Detail page missing 'Back to task list' button or link"
        );
    })
}

#[test]
fn tasks_filtered_by_label_are_displayed_correctly() {
    let mut rng = thread_rng();
    run_test!(|client, conn| {
        // Create new tasks
        let mut task_names: Vec<String> = Vec::with_capacity(3);
        let mut task_ids: Vec<i32> = Vec::with_capacity(3);
        for _ in 0..3 {
            let rng_name: String = (&mut rng)
                .sample_iter(&Alphanumeric)
                .map(char::from)
                .take(7)
                .collect();
            client
                .post("/")
                .header(ContentType::Form)
                .body(format!("name={}", rng_name))
                .dispatch()
                .await;
            let inserted_id = Task::all(&conn).await.last().unwrap().id.unwrap();
            task_names.push(rng_name);
            task_ids.push(inserted_id);
        }

        // Create a new label, too.
        client
            .post("/label")
            .header(ContentType::Form)
            .body("name=newlabel&color=#eeeeee".to_string())
            .dispatch()
            .await;
        let inserted_label_id = Label::all(&conn).await[0].id.unwrap();

        // Attach label to several tasks.
        let dt = Local::now().naive_local().to_string();
        for i in 0..2 {
            let form_data = format!(
                "name={}&description=&updated_at={}&label_id={}",
                &task_names[i], dt, inserted_label_id
            );
            client
                .post(format!("/{}", task_ids[i]))
                .header(ContentType::Form)
                .body(form_data)
                .dispatch()
                .await;
        }

        // Ensure several tasks are shown.
        let res = client
            .get(format!("/label/{}", inserted_label_id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        let body = res.into_string().await.unwrap();
        assert!(body.contains(&task_names[0]));
        assert!(body.contains(&task_names[1]));
        assert!(!body.contains(&task_names[2]));
    })
}

#[test]
fn task_delete_confirm_page_shows_buttons() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "confirmpagetest".to_string();
        client
            .post("/")
            .header(ContentType::Form)
            .body(format!("name={}", task_name))
            .dispatch()
            .await;
        let inserted_id = Task::all(&conn).await[0].id.unwrap();

        // Ensure we can access detail page.
        let res = client
            .get(format!("/{}/confirm", inserted_id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // Ensure confirm page shows buttons using HTML parser
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);

        // Check for Delete button
        let button_selector = Selector::parse("button[type='submit']").unwrap();
        let delete_button = document
            .select(&button_selector)
            .find(|el| el.text().any(|t| t.contains("Delete")));
        assert!(delete_button.is_some(), "Delete button not found!");

        // Check for Back to task button
        let back_button = document
            .select(&Selector::parse("button").unwrap())
            .find(|el| el.text().any(|t| t.contains("Back to task")));
        assert!(back_button.is_some(), "Back to task button not found!");

        // Check for onclick attribute for back navigation
        let has_onclick = document
            .select(&Selector::parse("button[onclick]").unwrap())
            .any(|el| {
                el.value()
                    .attr("onclick")
                    .map_or(false, |v| v.contains("location.href='/'"))
            });
        assert!(has_onclick, "Back button with onclick to '/' not found!");
    })
}

#[test]
fn label_delete_confirm_page_shows_buttons() {
    run_test!(|client, conn| {
        // Create a new label.
        client
            .post("/label")
            .header(ContentType::Form)
            .body("name=label+confirm+test&color=#ababab")
            .dispatch()
            .await;
        let inserted_id = Label::all(&conn).await[0].id.unwrap();

        // Ensure we can access confirm page.
        let res = client
            .get(format!("/label/{}/confirm", inserted_id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // Ensure confirm page shows buttons using HTML parser
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);

        // Check for Delete button
        let button_selector = Selector::parse("button[type='submit']").unwrap();
        let delete_button = document
            .select(&button_selector)
            .find(|el| el.text().any(|t| t.contains("Delete")));
        assert!(delete_button.is_some(), "Delete button not found!");

        // Check for Back to label button
        let back_button = document
            .select(&Selector::parse("button").unwrap())
            .find(|el| el.text().any(|t| t.contains("Back to label")));
        assert!(back_button.is_some(), "Back to label button not found!");

        // Check for onclick attribute for back navigation
        let has_onclick = document
            .select(&Selector::parse("button[onclick]").unwrap())
            .any(|el| {
                el.value()
                    .attr("onclick")
                    .map_or(false, |v| v.contains("location.href='/'"))
            });
        assert!(has_onclick, "Back button with onclick to '/' not found!");
    })
}

#[test]
fn task_insertion_and_deletion_updates_db_and_ui() {
    run_test!(|client, conn| {
        // Get the tasks before making changes.
        let init_tasks = Task::all(&conn).await;

        // insert new task
        client
            .post("/")
            .header(ContentType::Form)
            .body("name=test+task")
            .dispatch()
            .await;
        let time_posted_ndt = Local::now().naive_local();

        // Ensure we have one more task in the DB.
        let new_tasks = Task::all(&conn).await;
        assert_eq!(new_tasks.len(), init_tasks.len() + 1);

        // Ensure the task is what we expect.
        assert_eq!(new_tasks[0].name, "test task");
        assert_eq!(new_tasks[0].description, "");

        assert!(
            time_posted_ndt
                - NaiveDateTime::parse_from_str(&new_tasks[0].updated_at, "%Y-%m-%d %H:%M:%S%.f")
                    .unwrap()
                < Duration::seconds(5)
        );
        assert_eq!(new_tasks[0].label_id, None);

        // Delete task.
        let id = new_tasks[0].id.unwrap();
        client.delete(format!("/{}", id)).dispatch().await;

        // Ensure task was deleted.
        let final_tasks = Task::all(&conn).await;
        assert_eq!(final_tasks.len(), init_tasks.len());
        if !final_tasks.is_empty() {
            assert_ne!(final_tasks[0].name, "test task");
        }
    })
}

#[test]
fn label_insertion_and_deletion_updates_db_and_ui() {
    run_test!(|client, conn| {
        // Get the labels before making changes.
        let init_labels = Label::all(&conn).await;

        // Insert new label.
        client
            .post("/label")
            .header(ContentType::Form)
            .body("name=test+label&color=#ababab")
            .dispatch()
            .await;

        // Ensure we have one more label in the DB.
        let new_labels = Label::all(&conn).await;
        assert_eq!(new_labels.len(), init_labels.len() + 1);

        // Ensure the label is what we expect.
        assert_eq!(new_labels[0].name, "test label");
        assert_eq!(new_labels[0].color_hex, "#ababab");

        // Delete a label.
        let id = new_labels[0].id.unwrap();
        client.delete(format!("/label/{}", id)).dispatch().await;

        // Ensure label was deleted.
        let final_labels = Label::all(&conn).await;
        assert_eq!(final_labels.len(), init_labels.len());
        if !final_labels.is_empty() {
            assert_ne!(final_labels[0].name, "test label");
        }
    })
}

#[test]
fn inserting_many_tasks_displays_all_in_ui() {
    const ITER: usize = 100;

    let mut rng = thread_rng();
    run_test!(|client, conn| {
        let init_num = Task::all(&conn).await.len();
        let mut descs = Vec::new();

        for i in 0..ITER {
            // Insert new task with random name.
            let desc: String = (&mut rng)
                .sample_iter(&Alphanumeric)
                .map(char::from)
                .take(6)
                .collect();

            client
                .post("/")
                .header(ContentType::Form)
                .body(format!("name={}", desc))
                .dispatch()
                .await;

            // Record the name we choose for this iteration.
            descs.insert(0, desc);

            // Ensure the task was inserted properly and all other tasks remain.
            let tasks = Task::all_by_id(&conn).await;
            assert_eq!(tasks.len(), init_num + i + 1);

            for j in 0..i {
                assert_eq!(descs[j], tasks[j].name);
            }
        }
    })
}

#[test]
fn invalid_task_form_submission_shows_warnings() {
    run_test!(|client, _conn| {
        // Submit an **empty** form. This is an unexpected pattern
        // because task form in index page has `name` field.
        let res = client.post("/").header(ContentType::Form).dispatch().await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a name field. This is same as just above pattern.
        let res = client.post("/").header(ContentType::Form).dispatch().await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form with an empty name. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client
            .post("/")
            .header(ContentType::Form)
            .body("name=")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));
    })
}

#[test]
fn invalid_label_form_submission_shows_warnings() {
    run_test!(|client, _conn| {
        // Submit an **empty** form. This is an unexpected pattern
        // because label form in index page has `name` and `color` field.
        let res = client
            .post("/label")
            .header(ContentType::Form)
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a name field.
        let res = client
            .post("/label")
            .header(ContentType::Form)
            .body("color=#123456")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a color field.
        let res = client
            .post("/label")
            .header(ContentType::Form)
            .body("name=mylabel")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form with an empty name. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client
            .post("/label")
            .header(ContentType::Form)
            .body("name=&color=#ff00ff")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));

        // Submit a form with an empty color. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client
            .post("/label")
            .header(ContentType::Form)
            .body("name=mylabel&color=")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));

        // Submit a form with an invalid color. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client
            .post("/label")
            .header(ContentType::Form)
            .body("name=mylabel&color=red")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));

        // Submit a form with invalid color (color code has 7 digits).
        let res = client
            .post("/label")
            .header(ContentType::Form)
            .body("name=mylabel&color=#1234567")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));

        // Submit a form with invalid color (color code has 5 digits).
        let res = client
            .post("/label")
            .header(ContentType::Form)
            .body("name=mylabel&color=#12345")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));
    })
}

#[test]
fn invalid_task_update_form_submission_shows_warnings() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "detailformtest".to_string();
        client
            .post("/")
            .header(ContentType::Form)
            .body(format!("name={}", task_name))
            .dispatch()
            .await;
        let inserted_id = Task::all(&conn).await[0].id.unwrap();
        let post_url = format!("/{}", inserted_id);

        // Submit an **empty** form. This is an unexpected pattern
        // because task form in detail page has some fields.
        let res = client
            .post(&post_url)
            .header(ContentType::Form)
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a name field. This is same as just above pattern.
        let res = client
            .post(&post_url)
            .header(ContentType::Form)
            .body("description=hello")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form with an empty name. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = client
            .post(&post_url)
            .header(ContentType::Form)
            .body("name=&description=hello&updated_at=2020-04-28")
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));
    })
}

#[test]
fn updating_task_date_sets_to_today() {
    run_test!(|client, conn| {
        // Create new task with old `updated_at`.
        let mut rng = thread_rng();
        let rng_name: String = (&mut rng)
            .sample_iter(&Alphanumeric)
            .map(char::from)
            .take(7)
            .collect();

        let t = Task::insert_with_old_date(&rng_name, &conn).await;
        assert!(t);

        // Ensure `updated_at` of created task is updated to today.
        let new_tasks = Task::all(&conn).await;
        let today_ndt = Local::now().naive_local();
        // First, ensure current task date is not today.
        let new_task_nd = NaiveDate::parse_from_str(&new_tasks[0].updated_at, "%Y-%m-%d").unwrap();
        assert_ne!(new_task_nd, today_ndt.date());

        let inserted_id = new_tasks[0].id.unwrap(); // `id` is `Nullable`
        let res = client
            .post(format!("/{}/date", inserted_id))
            .dispatch()
            .await;
        let mut cookies = res.headers().get("Set-Cookie");
        let final_tasks = Task::all(&conn).await;
        let final_task_ndt =
            NaiveDateTime::parse_from_str(&final_tasks[0].updated_at, "%Y-%m-%d %H:%M:%S%.f")
                .unwrap();
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));
        assert_eq!(final_task_ndt.date(), today_ndt.date());
    })
}

#[test]
fn updating_task_fields_persists_changes() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name = "updatetasktest".to_string();
        let t = Task::insert_with_old_date(&task_name, &conn).await;
        assert!(t);

        // Create new label, too.
        client
            .post("/label")
            .header(ContentType::Form)
            .body("name=newlabel&color=#eeeeee".to_string())
            .dispatch()
            .await;

        // Submit valid update form.
        let inserted_id = Task::all(&conn).await[0].id.unwrap();
        let inserted_label_id = Label::all(&conn).await[0].id.unwrap();
        let task_description = "newdescription".to_string();
        let dt = Local::now().naive_local().to_string();
        let form_data = format!(
            "name={}&description={}&updated_at={}&label_id={}",
            task_name, task_description, dt, inserted_label_id
        );
        let res = client
            .post(format!("/{}", inserted_id))
            .header(ContentType::Form)
            .body(form_data)
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));

        let updated_task = Task::task_by_id(inserted_id, &conn).await;
        assert_eq!(updated_task.name, task_name);
        assert_eq!(updated_task.description, task_description);
        assert_eq!(updated_task.updated_at, dt);
        assert_eq!(updated_task.label_id, Some(inserted_label_id));

        // Update label_id to NULL.
        let form_data = format!(
            "name={}&description={}&updated_at={}&label_id=",
            task_name, task_description, dt
        );
        let res = client
            .post(format!("/{}", inserted_id))
            .header(ContentType::Form)
            .body(form_data)
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));

        let updated_task = Task::task_by_id(inserted_id, &conn).await;
        assert_eq!(updated_task.label_id, None);
    })
}

#[test]
fn updating_label_fields_persists_changes() {
    run_test!(|client, conn| {
        // Create a new label.
        client
            .post("/label")
            .header(ContentType::Form)
            .body("name=newlabel&color=#eeeeee".to_string())
            .dispatch()
            .await;
        let inserted_id = Label::all(&conn).await[0].id.unwrap();

        // Update above label.
        let new_name = "newnewlabel".to_string();
        let new_color = "#5566ff".to_string();
        let form_data = format!("name={}&color={}", &new_name, &new_color);
        let res = client
            .post(format!("/label/{}", inserted_id))
            .header(ContentType::Form)
            .body(form_data)
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));

        let updated_label = Label::label_by_id(inserted_id, &conn).await;
        assert_eq!(updated_label.name, new_name);
        assert_eq!(updated_label.color_hex, new_color);
    })
}
