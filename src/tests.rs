use super::models::label::Label;
use super::models::task::Task;

use parking_lot::{Mutex, const_mutex};
use rand::distr::{Alphanumeric, SampleString};

use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use scraper::{Html, Selector};

static DB_LOCK: Mutex<()> = const_mutex(());

macro_rules! run_test {
    (|$client:ident, $conn:ident| $block:expr_2021) => {{
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

// --- Test helpers ---

async fn insert_label_by_post<'a>(
    client: &'a Client,
    name: &'a str,
    color: &'a str,
) -> LocalResponse<'a> {
    client
        .post("/label")
        .header(ContentType::Form)
        .body(format!("name={}&color={}", name, color))
        .dispatch()
        .await
}

async fn insert_task_by_post<'a>(
    client: &'a Client,
    name: &'a str,
    description: &'a str,
    updated_at: &'a str,
    label_id: Option<i32>,
) -> LocalResponse<'a> {
    let mut form = format!(
        "name={}&description={}&updated_at={}",
        name, description, updated_at
    );
    if let Some(id) = label_id {
        form.push_str(&format!("&label_id={}", id));
    }
    client
        .post("/")
        .header(ContentType::Form)
        .body(form)
        .dispatch()
        .await
}

async fn label_exists(document: &Html, name: &str) -> bool {
    let selector = Selector::parse("td,li,span").unwrap();
    document
        .select(&selector)
        .any(|el| el.text().any(|t| t.trim() == name))
}

// --- Tests ---

#[test]
fn index_shows_main_task_table_headers() {
    run_test!(|client, _conn| {
        // --- Arrange: (No setup needed) ---

        // --- Act: Access index page ---
        let res = client.get("/").dispatch().await;
        // prerequisites for this test
        assert_eq!(res.status(), Status::Ok);
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let header_selector = Selector::parse("th").unwrap();
        let headers: Vec<String> = document
            .select(&header_selector)
            .map(|el| el.text().collect::<String>().trim().to_string())
            .collect();

        // --- Assert: Status and headers are correct ---
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
            "Task table header 'Update to today' missing"
        );
    })
}

#[test]
fn index_shows_update_to_today_button() {
    run_test!(|client, _conn| {
        // --- Arrange: Insert a new task and check index page is accessible ---
        insert_task_by_post(&client, "test+task", "", "", None).await;
        let res = client.get("/").dispatch().await;
        assert_eq!(res.status(), Status::Ok);

        // --- Act: Extract body and parse document ---
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);

        // --- Assert: The 'I did it today!' button is present in the table body ---
        let button_selector = Selector::parse("button, a").unwrap();
        let button_name = "I did it today!";
        let found = document
            .select(&button_selector)
            .any(|el| el.text().any(|t| t.trim() == button_name));
        assert!(found, "'{}' button not found in the table", button_name);
    })
}

#[test]
fn label_list_displays_created_labels() {
    run_test!(|client, conn| {
        // TODO: use rand for hex color code, too
        let mut rng = rand::rng();
        let name: String = Alphanumeric.sample_string(&mut rng, 6);

        // --- Arrange: Ensure label is absent ---
        let res = client.get("/label").dispatch().await;
        assert_eq!(
            res.status(),
            Status::Ok,
            "Label list page returned non-200 status"
        );
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        // Assert that label name is NOT present before creation
        assert!(
            !label_exists(&document, &name).await,
            "Label name '{}' unexpectedly found in label list page",
            name
        );

        // --- Act: Create the label ---
        insert_label_by_post(&client, &name, "#ababab").await;

        // --- Assert: Label is present ---
        let res = client.get("/label").dispatch().await;
        assert_eq!(
            res.status(),
            Status::Ok,
            "Label list page returned non-200 status after label creation"
        );
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        // Assert that label name IS present after creation
        assert!(
            label_exists(&document, &name).await,
            "Label name '{}' not found in label list page after creation",
            name
        );
    })
}

#[test]
fn task_detail_page_shows_required_fields() {
    run_test!(|client, conn| {
        // Create new task and get its ID using helper.
        let task_name = "detailpagetest";
        let task_description = "desc";
        let updated_at = Local::now().naive_local().to_string();
        insert_task_by_post(&client, task_name, task_description, &updated_at, None).await;
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
        let label_texts = ["Label", "Task name", "Description", "Last updated"];
        for expected in &label_texts {
            assert!(
                body.contains(expected),
                "Detail page missing '{}' field",
                expected
            );
        }
    })
}

#[test]
fn task_detail_page_shows_update_button() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "detailpagetest".to_string();
        insert_task_by_post(&client, &task_name, "", "", None).await;
        let inserted_id = Task::all(&conn).await[0].id.unwrap();

        // Ensure we can access detail page.
        let res = client.get(format!("/{}", inserted_id)).dispatch().await;
        assert_eq!(
            res.status(),
            Status::Ok,
            "Detail page returned non-200 status"
        );

        // Ensure detail page shows Update button.
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let update_button = document
            .select(&Selector::parse("button[type='submit']").unwrap())
            .find(|el| el.text().any(|t| t.contains("Update")));
        assert!(
            update_button.is_some(),
            "Detail page missing 'Update' button"
        );
    })
}

#[test]
fn task_detail_page_shows_back_button() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "detailpagetest".to_string();
        insert_task_by_post(&client, &task_name, "", "", None).await;
        let inserted_id = Task::all(&conn).await[0].id.unwrap();

        // Ensure we can access detail page.
        let res = client.get(format!("/{}", inserted_id)).dispatch().await;
        assert_eq!(
            res.status(),
            Status::Ok,
            "Detail page returned non-200 status"
        );

        // Ensure detail page shows Back button or link.
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let back_button = document
            .select(&Selector::parse("button[onclick],a[onclick]").unwrap())
            .find(|el| {
                el.value()
                    .attr("onclick")
                    .is_some_and(|v| v.contains("location.href='../'"))
            });
        assert!(
            back_button.is_some(),
            "Detail page missing 'Back to task list' button or link"
        );
    })
}

#[test]
fn tasks_filtered_by_label_are_displayed_correctly() {
    let mut rng = rand::rng();
    run_test!(|client, conn| {
        // Create new tasks
        let mut task_names: Vec<String> = Vec::with_capacity(3);
        let mut task_ids: Vec<i32> = Vec::with_capacity(3);
        for _ in 0..3 {
            let rng_name: String = Alphanumeric.sample_string(&mut rng, 7);
            insert_task_by_post(&client, &rng_name, "", "", None).await;
            let inserted_id = Task::all(&conn).await.last().unwrap().id.unwrap();
            task_names.push(rng_name);
            task_ids.push(inserted_id);
        }

        // Create a new label, too.
        insert_label_by_post(&client, "newlabel", "#eeeeee").await;
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
fn task_delete_confirm_page_shows_delete_button() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "confirmpagetest".to_string();
        insert_task_by_post(&client, &task_name, "", "", None).await;
        let inserted_id = Task::all(&conn).await[0].id.unwrap();

        // Ensure we can access confirm page.
        let res = client
            .get(format!("/{}/confirm", inserted_id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // Ensure confirm page shows Delete button using HTML parser
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let button_selector = Selector::parse("button[type='submit']").unwrap();
        let delete_button = document
            .select(&button_selector)
            .find(|el| el.text().any(|t| t.contains("Delete")));
        assert!(delete_button.is_some(), "Delete button not found!");
    })
}

#[test]
fn task_delete_confirm_page_shows_back_to_task_button() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "confirmpagetest".to_string();
        insert_task_by_post(&client, &task_name, "", "", None).await;
        let inserted_id = Task::all(&conn).await[0].id.unwrap();

        // Ensure we can access confirm page.
        let res = client
            .get(format!("/{}/confirm", inserted_id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // Ensure confirm page shows Back to task button using HTML parser
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let back_button = document
            .select(&Selector::parse("button").unwrap())
            .find(|el| el.text().any(|t| t.contains("Back to task")));
        assert!(back_button.is_some(), "Back to task button not found!");
    })
}

#[test]
fn task_delete_confirm_page_shows_back_to_index_button() {
    run_test!(|client, conn| {
        // Create new task and get its ID.
        let task_name: String = "confirmpagetest".to_string();
        insert_task_by_post(&client, &task_name, "", "", None).await;
        let inserted_id = Task::all(&conn).await[0].id.unwrap();

        // Ensure we can access confirm page.
        let res = client
            .get(format!("/{}/confirm", inserted_id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // Ensure Back button has onclick to root page ('/')
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let has_onclick = document
            .select(&Selector::parse("button[onclick]").unwrap())
            .any(|el| {
                el.value()
                    .attr("onclick")
                    .is_some_and(|v| v.contains("location.href='/'"))
            });
        assert!(has_onclick, "Back button with onclick to '/' not found!");
    })
}

#[test]
fn label_delete_confirm_page_shows_delete_button() {
    run_test!(|client, conn| {
        // Create a new label.
        insert_label_by_post(&client, "labelconfirmtest", "#ababab").await;
        let inserted_id = Label::all(&conn).await[0].id.unwrap();

        // Ensure we can access confirm page.
        let res = client
            .get(format!("/label/{}/confirm", inserted_id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // Ensure confirm page shows Delete button using HTML parser
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let button_selector = Selector::parse("button[type='submit']").unwrap();
        let delete_button = document
            .select(&button_selector)
            .find(|el| el.text().any(|t| t.contains("Delete")));
        assert!(delete_button.is_some(), "Delete button not found!");
    })
}

#[test]
fn label_delete_confirm_page_shows_back_to_label_button() {
    run_test!(|client, conn| {
        // Create a new label.
        insert_label_by_post(&client, "labelconfirmtest", "#ababab").await;
        let inserted_id = Label::all(&conn).await[0].id.unwrap();

        // Ensure we can access confirm page.
        let res = client
            .get(format!("/label/{}/confirm", inserted_id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // Ensure confirm page shows Back to label button using HTML parser
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let back_button = document
            .select(&Selector::parse("button").unwrap())
            .find(|el| el.text().any(|t| t.contains("Back to label")));
        assert!(back_button.is_some(), "Back to label button not found!");
    })
}

#[test]
fn label_delete_confirm_page_shows_back_to_index_button() {
    run_test!(|client, conn| {
        // Create a new label.
        insert_label_by_post(&client, "labelconfirmtest", "#ababab").await;
        let inserted_id = Label::all(&conn).await[0].id.unwrap();

        // Ensure we can access confirm page.
        let res = client
            .get(format!("/label/{}/confirm", inserted_id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // Ensure Back button has onclick to root page ('/')
        let body = res.into_string().await.unwrap();
        let document = Html::parse_document(&body);
        let has_onclick = document
            .select(&Selector::parse("button[onclick]").unwrap())
            .any(|el| {
                el.value()
                    .attr("onclick")
                    .is_some_and(|v| v.contains("location.href='/'"))
            });
        assert!(has_onclick, "Back button with onclick to '/' not found!");
    })
}

#[test]
fn task_insertion_and_deletion_updates_db_and_ui() {
    run_test!(|client, conn| {
        // --- Arrange: Get initial tasks ---
        let init_tasks = Task::all(&conn).await;

        // --- Act: Insert new task ---
        insert_task_by_post(&client, "test task", "", "", None).await;
        let time_posted_ndt = Local::now().naive_local();

        // --- Assert: Task inserted in DB ---
        let new_tasks = Task::all(&conn).await;
        assert_eq!(new_tasks.len(), init_tasks.len() + 1);
        assert_eq!(new_tasks[0].name, "test task");
        assert_eq!(new_tasks[0].description, "");
        assert!(
            time_posted_ndt
                - NaiveDateTime::parse_from_str(&new_tasks[0].updated_at, "%Y-%m-%d %H:%M:%S%.f")
                    .unwrap()
                < Duration::seconds(5)
        );
        assert_eq!(new_tasks[0].label_id, None);

        // --- Act: Delete the task ---
        let id = new_tasks[0].id.unwrap();
        client.delete(format!("/{}", id)).dispatch().await;

        // --- Assert: Task deleted from DB ---
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
        // --- Arrange: Get initial labels ---
        let init_labels = Label::all(&conn).await;

        // --- Act: Insert new label ---
        insert_label_by_post(&client, "test label", "#ababab").await;

        // --- Assert: Label inserted in DB ---
        let new_labels = Label::all(&conn).await;
        assert_eq!(new_labels.len(), init_labels.len() + 1);
        assert_eq!(new_labels[0].name, "test label");
        assert_eq!(new_labels[0].color_hex, "#ababab");

        // --- Act: Delete the label ---
        let id = new_labels[0].id.unwrap();
        client.delete(format!("/label/{}", id)).dispatch().await;

        // --- Assert: Label deleted from DB ---
        let final_labels = Label::all(&conn).await;
        assert_eq!(final_labels.len(), init_labels.len());
        if !final_labels.is_empty() {
            assert_ne!(final_labels[0].name, "test label");
        }
    })
}

#[test]
fn inserting_multiple_tasks_preserves_insertion_order() {
    const TEST_RECORD_NUM: usize = 100;

    run_test!(|client, conn| {
        // --- Arrange: prepare variables for later assertions ---
        let mut rng = rand::rng();
        let mut inserted_names = Vec::new();

        // --- Act: Insert many tasks ---
        for _ in 0..TEST_RECORD_NUM {
            let name: String = Alphanumeric.sample_string(&mut rng, 6);
            insert_task_by_post(&client, &name, "", "", None).await;
            inserted_names.push(name);
        }
        let tasks = Task::all_by_id(&conn).await;
        // all tasks inserted?
        assert_eq!(tasks.len(), TEST_RECORD_NUM);

        // --- Assert: inserted tasks are in the same order as inserted. ---
        for i in 0..TEST_RECORD_NUM {
            assert_eq!(tasks[i].name, inserted_names[i]);
        }
    })
}

#[test]
fn invalid_task_form_submission_shows_warnings() {
    run_test!(|client, _conn| {
        // Submit POST request without a form. This is an unexpected pattern
        // because task form in index page has `name` field.
        let res = client.post("/").header(ContentType::Form).dispatch().await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form with an empty name. We look for `warning` in the
        // cookies which corresponds to flash message being set as a warning.
        let res = insert_task_by_post(&client, "", "", "", None).await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));
    })
}

#[test]
fn invalid_label_form_submission_shows_warnings() {
    run_test!(|client, _conn| {
        // Submit POST request without a form. This is an unexpected pattern
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
        insert_label_by_post(&client, "mylabel", "#1234567").await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("warning")));

        // Submit a form with invalid color (color code has 5 digits).
        insert_label_by_post(&client, "mylabel", "#12345").await;

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
        insert_task_by_post(&client, &task_name, "", "", None).await;
        let inserted_id = Task::all(&conn).await[0].id.unwrap();
        let post_url = format!("/{}", inserted_id);

        // Submit POST request without a form. This is an unexpected pattern
        // because task form in detail page has some fields.
        let res = client
            .post(&post_url)
            .header(ContentType::Form)
            .dispatch()
            .await;

        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::UnprocessableEntity);
        assert!(!cookies.any(|value| value.contains("warning")));

        // Submit a form without a name field.
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
        let mut rng = rand::rng();
        let rng_name: String = Alphanumeric.sample_string(&mut rng, 7);

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
        // --- Arrange: Create new task and label ---
        let task_name = "updatetasktest".to_string();
        let t = Task::insert_with_old_date(&task_name, &conn).await;
        assert!(t);

        insert_label_by_post(&client, "newlabel", "#eeeeee").await;

        // --- Act: Submit valid update form ---
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

        // --- Assert: DB and UI reflect changes ---
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
        // --- Arrange: Create a new label ---
        insert_label_by_post(&client, "newlabel", "#eeeeee").await;
        let inserted_id = Label::all(&conn).await[0].id.unwrap();

        // --- Act: Update the label ---
        let new_name = "newnewlabel".to_string();
        let new_color = "#5566ff".to_string();
        let form_data = format!("name={}&color={}", &new_name, &new_color);
        let res = client
            .post(format!("/label/{}", inserted_id))
            .header(ContentType::Form)
            .body(form_data)
            .dispatch()
            .await;

        // --- Assert: DB and UI reflect changes ---
        let mut cookies = res.headers().get("Set-Cookie");
        assert_eq!(res.status(), Status::SeeOther);
        assert!(cookies.any(|value| value.contains("success")));

        let updated_label = Label::label_by_id(inserted_id, &conn).await;
        assert_eq!(updated_label.name, new_name);
        assert_eq!(updated_label.color_hex, new_color);
    })
}
