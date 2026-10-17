CREATE TABLE review_sessions (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL CHECK(status IN ('active','completed','aborted')),
    record TEXT NOT NULL
);
CREATE UNIQUE INDEX one_active_review ON review_sessions(status) WHERE status='active';
CREATE TABLE review_xp (
    study_date TEXT PRIMARY KEY,
    session_id TEXT NOT NULL UNIQUE REFERENCES review_sessions(id),
    amount INTEGER NOT NULL CHECK(amount=5)
);
PRAGMA user_version=3;
