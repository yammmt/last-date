-- Task id will be changed.

ALTER TABLE tasks RENAME TO tmp;
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR NOT NULL,
    description VARCHAR NOT NULL
);
INSERT INTO tasks(name, description) SELECT name, description FROM tmp;
DROP TABLE tmp;
