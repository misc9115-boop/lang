CREATE TABLE pack_versions (
    pack_id TEXT NOT NULL,
    version INTEGER NOT NULL CHECK(version > 0),
    digest TEXT NOT NULL,
    manifest BLOB NOT NULL,
    stored INTEGER NOT NULL CHECK(stored IN (0,1)),
    payload_bytes INTEGER NOT NULL,
    installed_at TEXT NOT NULL,
    PRIMARY KEY(pack_id,version)
);
CREATE TABLE pack_files (
    pack_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    path TEXT NOT NULL,
    bytes BLOB NOT NULL,
    PRIMARY KEY(pack_id,version,path),
    FOREIGN KEY(pack_id,version) REFERENCES pack_versions(pack_id,version)
);
CREATE TABLE active_packs (
    pack_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL,
    FOREIGN KEY(pack_id,version) REFERENCES pack_versions(pack_id,version)
);
CREATE TABLE identities (
    pack_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    content_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    digest TEXT NOT NULL,
    PRIMARY KEY(pack_id,kind,content_id,version)
);
PRAGMA application_id = 1280330545;
PRAGMA user_version = 1;
