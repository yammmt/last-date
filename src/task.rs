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
use self::schema::tasks::dsl::tasks as all_tasks;

use crate::label::Label;

#[table_name="tasks"]
#[belongs_to(Label, foreign_key = "label_id")]
#[derive(Associations, Identifiable, Serialize, Queryable, Insertable, Debug, Clone)]
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
    pub label_id: Option<i32>
}

impl Task {
    pub fn all(conn: &SqliteConnection) -> Vec<Task> {
        // Task hasn't been done for a long time should be in the top.
        all_tasks.order(tasks::updated_at.asc()).load::<Task>(conn).unwrap()
    }

    #[cfg(test)]
    pub fn all_by_id(conn: &SqliteConnection) -> Vec<Task> {
        // I don't know why sometimes `all` called by `test_many_insertions`
        // invites SIGSEGV: invalid memory reference error...
        all_tasks.order(tasks::id.desc()).load::<Task>(conn).unwrap()
    }

    pub fn task_by_id(id: i32, conn: &SqliteConnection) -> Task {
        all_tasks.find(id).load::<Task>(conn).unwrap().first().unwrap().clone()
    }

    pub fn tasks_by_label(label_id: i32, conn: &SqliteConnection) -> Vec<Task> {
        let label = Label::label_by_id(label_id, conn);
        Task::belonging_to(&label).order(tasks::name).load::<Task>(conn).unwrap()
    }

    pub fn insert(task_name: TaskName, conn: &SqliteConnection) -> bool {
        let dt = Local::today().naive_local();
        let t = Task { id: None, name: task_name.name, description: "".to_string(), updated_at: dt.to_string(), label_id: None };
        diesel::insert_into(tasks::table).values(&t).execute(conn).is_ok()
    }

    #[cfg(test)]
    pub fn insert_with_old_date(dummy_name: &str, conn: &SqliteConnection) -> bool {
        let t = Task { id: None, name: dummy_name.to_string(), description: "".to_string(), updated_at: "2000-01-01".to_string(), label_id: None };
        diesel::insert_into(tasks::table).values(&t).execute(conn).is_ok()
    }

    // Via this function, `updated_at` isn't updated because both task name and
    // description don't change the date its task was done.
    pub fn update(id: i32, task: TaskUpdate, conn: &SqliteConnection) -> bool {
        diesel::update(all_tasks.find(id))
            .set((
                tasks::name.eq(task.name),
                tasks::description.eq(task.description),
                tasks::label_id.eq(task.label_id)
            )).execute(conn).is_ok()
    }

    pub fn update_to_today(id: i32, conn: &SqliteConnection) -> bool {
        let dt = Local::today().naive_local();
        diesel::update(all_tasks.find(id)).set(tasks::updated_at.eq(dt.to_string())).execute(conn).is_ok()
    }

    pub fn delete_with_id(id: i32, conn: &SqliteConnection) -> bool {
        diesel::delete(all_tasks.find(id)).execute(conn).is_ok()
    }

    #[cfg(test)]
    pub fn delete_all(conn: &SqliteConnection) -> bool {
        diesel::delete(all_tasks).execute(conn).is_ok()
    }
}
