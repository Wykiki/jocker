CREATE TABLE metadata (
  id                  BIGINT NOT NULL PRIMARY KEY,
  binaries_updated_at DATETIME,
  config_updated_at   DATETIME,
  default_stack       TEXT REFERENCES stack(name) ON DELETE SET NULL
);

CREATE TABLE binary (
  name  TEXT NOT NULL PRIMARY KEY,
  id    TEXT NOT NULL
);

CREATE TABLE process (
  name        TEXT NOT NULL PRIMARY KEY,
  binary      TEXT NOT NULL,
  state       TEXT NOT NULL,
  pid         INTEGER,
  args        TEXT NOT NULL,
  cargo_args  TEXT NOT NULL,
  env         TEXT NOT NULL
);

CREATE TABLE stack (
  name    TEXT NOT NULL PRIMARY KEY
);

CREATE TABLE rel_stack_process (
  stack_name    TEXT NOT NULL REFERENCES stack(name) ON DELETE CASCADE,
  process_name  TEXT NOT NULL REFERENCES process(name) ON DELETE CASCADE
);
CREATE INDEX idx_stack_name   ON rel_stack_process (stack_name);
CREATE INDEX idx_process_name ON rel_stack_process (process_name);

CREATE TABLE rel_stack_inherited_process (
  stack_name    TEXT NOT NULL REFERENCES stack(name) ON DELETE CASCADE,
  process_name  TEXT NOT NULL REFERENCES process(name) ON DELETE CASCADE
);
CREATE INDEX idx_inherited_stack_name   ON rel_stack_process (stack_name);
CREATE INDEX idx_inherited_process_name ON rel_stack_process (process_name);
