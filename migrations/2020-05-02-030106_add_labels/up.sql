CREATE table labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR,
    color_hex VARCHAR
);
INSERT INTO labels(name, color_hex) VALUES ("cleaning", "#b0c4de");
INSERT INTO labels(name, color_hex) VALUES ("washing", "#faf0e6");

ALTER TABLE tasks RENAME TO tmp;
PRAGMA foreign_keys = ON;
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    updated_at VARCHAR NOT NULL DEFAULT "2020-01-01",
    label_id INTEGER,
    FOREIGN KEY (label_id) REFERENCES labels (id) ON DELETE SET NULL ON UPDATE CASCADE
);
INSERT INTO tasks(name, description, updated_at) SELECT name, description, updated_at FROM tmp;

DROP TABLE tmp;
