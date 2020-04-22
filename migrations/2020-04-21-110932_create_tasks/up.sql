CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR NOT NULL,
    description VARCHAR NOT NULL
);

INSERT INTO tasks (name, description) VALUES ("foo", "demo task");
INSERT INTO tasks (name, description) VALUES ("bar", "");
