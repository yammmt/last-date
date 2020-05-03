use diesel::{self, prelude::*};

mod schema {
    table! {
        labels {
            id -> Nullable<Integer>,
            name -> Text,
            color_hex -> Text,
        }
    }
}

use self::schema::labels;
use self::schema::labels::dsl::labels as all_labels;

#[table_name="labels"]
#[derive(Identifiable, Serialize, Queryable, Insertable, Debug, Clone)]
pub struct Label {
    pub id: Option<i32>,
    pub name: String,
    pub color_hex: String,
}

impl Label {
    pub fn all(conn: &SqliteConnection) -> Vec<Label> {
        // TODO: order by name? id?
        all_labels.order(labels::id).load::<Label>(conn).unwrap()
    }

    pub fn label_by_id(id: i32, conn: &SqliteConnection) -> Label {
        all_labels.find(id).load::<Label>(conn).unwrap().first().unwrap().clone()
    }
}
