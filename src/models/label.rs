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

#[derive(FromForm)]
pub struct LabelForm {
    pub name: String,
    pub color: String,
}

impl Label {
    pub fn all(conn: &SqliteConnection) -> Vec<Label> {
        all_labels.order(labels::name).load::<Label>(conn).unwrap()
    }

    pub fn label_by_id(id: i32, conn: &SqliteConnection) -> Label {
        all_labels.find(id).load::<Label>(conn).unwrap().first().unwrap().clone()
    }

    pub fn insert(label_info: LabelForm, conn: &SqliteConnection) -> bool {
        let l = Label { id: None, name: label_info.name, color_hex: label_info.color };
        diesel::insert_into(labels::table).values(&l).execute(conn).is_ok()
    }

    pub fn update(id: i32, label: LabelForm, conn: &SqliteConnection) -> bool {
        diesel::update(all_labels.find(id))
            .set((
                labels::name.eq(label.name),
                labels::color_hex.eq(label.color)
            )).execute(conn).is_ok()
    }

    pub fn delete_with_id(id: i32, conn: &SqliteConnection) -> bool {
        diesel::delete(all_labels.find(id)).execute(conn).is_ok()
    }

    #[cfg(test)]
    pub fn delete_all(conn: &SqliteConnection) -> bool {
        diesel::delete(all_labels).execute(conn).is_ok()
    }
}
