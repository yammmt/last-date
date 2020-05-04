ALTER TABLE tasks RENAME to tmp;
CREATE TABLE tasks {
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    updated_at VARCHAR NOT NULL DEFAULT "2020-01-01"
};
INSERT INTO tasks(name, description, updated_at) SELECT name, description, updated_at FROM tmp;
DROP TABLE tmp;
DROP TABLE labels;
