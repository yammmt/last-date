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

use crate::DbConn;

use self::schema::labels;

#[derive(Identifiable, Serialize, Queryable, Insertable, Debug, Clone)]
#[diesel(table_name = labels)]
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
    pub async fn all(conn: &DbConn) -> Vec<Label> {
        conn.run(|c| {
            labels::table
                .order(labels::name)
                .load::<Label>(c)
                .unwrap_or_default()
        })
        .await
    }

    pub async fn label_by_id(id: i32, conn: &DbConn) -> Label {
        // TODO: avoid cloning `String`: could be slow
        conn.run(move |c| {
            labels::table
                .filter(labels::id.eq(id))
                .load::<Label>(c)
                .unwrap()
                .first()
                .unwrap()
                .clone()
        })
        .await
    }

    pub async fn insert(label_info: LabelForm, conn: &DbConn) -> bool {
        conn.run(|c| {
            let l = Label {
                id: None,
                name: label_info.name,
                color_hex: label_info.color,
            };
            diesel::insert_into(labels::table)
                .values(&l)
                .execute(c)
                .is_ok()
        })
        .await
    }

    pub async fn update(id: i32, label: LabelForm, conn: &DbConn) -> bool {
        conn.run(move |c| {
            diesel::update(labels::table.filter(labels::id.eq(id)))
                .set((
                    labels::name.eq(label.name),
                    labels::color_hex.eq(label.color),
                ))
                .execute(c)
                .is_ok()
        })
        .await
    }

    pub async fn delete_with_id(id: i32, conn: &DbConn) -> bool {
        conn.run(move |c| {
            diesel::delete(labels::table.filter(labels::id.eq(id)))
                .execute(c)
                .is_ok()
        })
        .await
    }

    #[cfg(test)]
    pub async fn delete_all(conn: &DbConn) -> bool {
        conn.run(|c| diesel::delete(labels::table).execute(c).is_ok())
            .await
    }
}
