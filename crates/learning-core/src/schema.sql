CREATE TABLE profile (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    settings TEXT NOT NULL
);
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    lesson_id TEXT NOT NULL,
    lesson_version INTEGER NOT NULL,
    lesson_snapshot TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL CHECK(status IN ('active', 'completed')),
    practice_only INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    xp_earned INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX one_active_session ON sessions(status) WHERE status = 'active';
CREATE TABLE session_items (
    session_id TEXT NOT NULL REFERENCES sessions(id),
    position INTEGER NOT NULL,
    draft TEXT,
    first_correct INTEGER,
    first_independent INTEGER,
    hinted INTEGER NOT NULL DEFAULT 0,
    revealed INTEGER NOT NULL DEFAULT 0,
    resolved INTEGER NOT NULL DEFAULT 0,
    feedback TEXT,
    PRIMARY KEY(session_id, position)
);
CREATE TABLE attempts (
    request_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    position INTEGER NOT NULL,
    correct INTEGER NOT NULL,
    hinted INTEGER NOT NULL,
    revealed INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE xp_ledger (
    id TEXT PRIMARY KEY,
    session_id TEXT UNIQUE NOT NULL REFERENCES sessions(id),
    lesson_id TEXT NOT NULL,
    study_date TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK(amount = 10),
    UNIQUE(lesson_id, study_date)
);
CREATE TABLE study_days (
    date TEXT PRIMARY KEY,
    time_zone TEXT NOT NULL,
    goal INTEGER NOT NULL CHECK(goal > 0)
);
CREATE TABLE achievements (
    id TEXT PRIMARY KEY,
    granted_at TEXT NOT NULL
);
CREATE TABLE answer_reports (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    exercise_id TEXT NOT NULL,
    note TEXT NOT NULL,
    created_at TEXT NOT NULL
);
PRAGMA user_version = 1;
