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
        }
    }
}

use self::schema::tasks;
use self::schema::tasks::dsl::tasks as all_tasks;

#[table_name="tasks"]
#[derive(Serialize, Queryable, Insertable, Debug, Clone)]
pub struct Task {
    pub id: Option<i32>,
    pub name: String,
    pub description: String,
    pub updated_at: String,
}

#[derive(FromForm)]
pub struct TaskName {
    pub name: String,
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

    pub fn insert(task_name: TaskName, conn: &SqliteConnection) -> bool {
        let dt = Local::today().naive_local();
        let t = Task { id: None, name: task_name.name, description: "".to_string(), updated_at: dt.to_string() };
        diesel::insert_into(tasks::table).values(&t).execute(conn).is_ok()
    }

    pub fn delete_with_id(id: i32, conn: &SqliteConnection) -> bool {
        diesel::delete(all_tasks.find(id)).execute(conn).is_ok()
    }

    #[cfg(test)]
    pub fn delete_all(conn: &SqliteConnection) -> bool {
        diesel::delete(all_tasks).execute(conn).is_ok()
    }
}
