use chrono::Local;
use diesel::{self, prelude::*};

mod schema {
    table! {
        tasks {
            // TODO: allow Nullable description
            id -> Nullable<Integer>, // primary key
            name -> Text,
            description -> Text,
            updated_at -> Timestamp,
            label_id -> Nullable<Integer>, // foreign key
        }
    }
}

use self::schema::tasks;

use crate::DbConn;
use crate::models::label::Label;

#[derive(Associations, Identifiable, Serialize, Queryable, Insertable, Debug, Clone)]
#[diesel(table_name = tasks)]
#[diesel(belongs_to(Label, foreign_key = label_id))]
pub struct Task {
    pub id: Option<i32>,
    pub name: String,
    pub description: String,
    pub updated_at: String,
    pub label_id: Option<i32>,
}

#[derive(FromForm)]
pub struct TaskName {
    pub name: String,
}

#[derive(FromForm)]
pub struct TaskUpdate {
    pub name: String,
    pub description: String,
    pub updated_at: String,
    pub label_id: Option<i32>,
}

impl Task {
    pub async fn all(conn: &DbConn) -> Vec<Task> {
        // Task hasn't been done for a long time should be in the top.
        conn.run(|c| {
            tasks::table
                .order(tasks::updated_at.asc())
                .load::<Task>(c)
                .unwrap_or_default()
        })
        .await
    }

    #[cfg(test)]
    pub async fn all_by_id(conn: &DbConn) -> Vec<Task> {
        // I don't know why sometimes `all` called by `test_many_insertions`
        // invites SIGSEGV: invalid memory reference error...
        conn.run(|c| tasks::table.order(tasks::id.asc()).load::<Task>(c).unwrap())
            .await
    }

    pub async fn task_by_id(id: i32, conn: &DbConn) -> Task {
        conn.run(move |c| {
            tasks::table
                .filter(tasks::id.eq(id))
                .load::<Task>(c)
                .unwrap()
                .first()
                .unwrap()
                .clone()
        })
        .await
    }

    pub async fn tasks_by_label(label_id: i32, conn: &DbConn) -> Vec<Task> {
        let label = Label::label_by_id(label_id, conn).await;
        conn.run(move |c| {
            Task::belonging_to(&label)
                .order(tasks::name)
                .load::<Task>(c)
                .unwrap()
        })
        .await
    }

    pub async fn insert(task_name: TaskName, conn: &DbConn) -> bool {
        let dt = Local::now().naive_local();
        let t = Task {
            id: None,
            name: task_name.name,
            description: "".to_string(),
            updated_at: dt.to_string(),
            label_id: None,
        };
        conn.run(move |c| {
            diesel::insert_into(tasks::table)
                .values(&t)
                .execute(c)
                .is_ok()
        })
        .await
    }

    #[cfg(test)]
    pub async fn insert_with_old_date(dummy_name: &str, conn: &DbConn) -> bool {
        let t = Task {
            id: None,
            name: dummy_name.to_string(),
            description: "".to_string(),
            updated_at: "2000-01-01".to_string(),
            label_id: None,
        };
        conn.run(move |c| {
            diesel::insert_into(tasks::table)
                .values(&t)
                .execute(c)
                .is_ok()
        })
        .await
    }

    pub async fn update(id: i32, task: TaskUpdate, conn: &DbConn) -> bool {
        conn.run(move |c| {
            diesel::update(tasks::table.filter(tasks::id.eq(id)))
                .set((
                    tasks::name.eq(task.name),
                    tasks::description.eq(task.description),
                    tasks::updated_at.eq(task.updated_at),
                    tasks::label_id.eq(task.label_id),
                ))
                .execute(c)
                .is_ok()
        })
        .await
    }

    pub async fn update_to_today(id: i32, conn: &DbConn) -> bool {
        let dt = Local::now().naive_local();
        conn.run(move |c| {
            diesel::update(tasks::table.filter(tasks::id.eq(id)))
                .set(tasks::updated_at.eq(dt.to_string()))
                .execute(c)
                .is_ok()
        })
        .await
    }

    pub async fn delete_with_id(id: i32, conn: &DbConn) -> bool {
        conn.run(move |c| {
            diesel::delete(tasks::table.filter(tasks::id.eq(id)))
                .execute(c)
                .is_ok()
        })
        .await
    }

    #[cfg(test)]
    pub async fn delete_all(conn: &DbConn) -> bool {
        conn.run(|c| diesel::delete(tasks::table).execute(c).is_ok())
            .await
    }
}
