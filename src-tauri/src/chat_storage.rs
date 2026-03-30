//! Persistent chat history backed by SQLite (WAL mode).
//!
//! The database lives at:
//!   `{app_data_dir}/chats/chats.db`
//!
//! which mirrors the pattern used by model storage — a dedicated sub-folder
//! so the file is easy to locate, back up, or wipe independently.
//!
//! # Schema
//!
//! ```sql
//! conversations(id TEXT PK, model_id TEXT, model_filename TEXT,
//!               backend TEXT, title TEXT, created_at INTEGER, updated_at INTEGER)
//!
//! messages(id INTEGER PK, conversation_id TEXT FK, role TEXT,
//!          content TEXT, created_at INTEGER)
//! ```
//!
//! # WAL mode
//!
//! `PRAGMA journal_mode = WAL` is set immediately after every `Connection::open`
//! call. WAL gives concurrent readers, faster writes, and crash safety at the
//! cost of one extra `-wal` / `-shm` sidecar file — a fine trade for a desktop
//! app that does frequent appends.

use std::path::PathBuf;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

// ── Path helper ───────────────────────────────────────────────────────────────

/// Returns (and creates if necessary) the path to the SQLite database.
///
/// Layout: `{app_data_dir}/chats/chats.db`
fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {e}"))?
        .join("chats");

    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Could not create chats directory '{}': {e}", dir.display()))?;

    Ok(dir.join("chats.db"))
}

// ── Connection helper ─────────────────────────────────────────────────────────

/// Opens (or creates) the database and immediately enables WAL mode +
/// foreign-key enforcement, then runs the schema migration.
fn open_db(app: &AppHandle) -> Result<Connection, String> {
    let path = db_path(app)?;
    let conn = Connection::open(&path)
        .map_err(|e| format!("Could not open database '{}': {e}", path.display()))?;

    // WAL mode — set before anything else.
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;",
    )
    .map_err(|e| format!("Could not set PRAGMAs: {e}"))?;

    migrate(&conn)?;
    Ok(conn)
}

// ── Schema migration ──────────────────────────────────────────────────────────

/// Idempotent schema setup.  Add new `ALTER TABLE` statements here for future
/// versions — the `IF NOT EXISTS` guards make all statements safe to re-run.
fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS conversations (
            id             TEXT    PRIMARY KEY NOT NULL,
            model_id       TEXT    NOT NULL DEFAULT '',
            model_filename TEXT    NOT NULL DEFAULT '',
            backend        TEXT    NOT NULL DEFAULT '',
            title          TEXT    NOT NULL DEFAULT 'New chat',
            created_at     INTEGER NOT NULL,
            updated_at     INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS messages (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id TEXT    NOT NULL
                REFERENCES conversations(id) ON DELETE CASCADE,
            role            TEXT    NOT NULL,
            content         TEXT    NOT NULL,
            created_at      INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_messages_conversation
            ON messages(conversation_id, created_at);",
    )
    .map_err(|e| format!("Schema migration failed: {e}"))
}

// ── Public types ──────────────────────────────────────────────────────────────

/// A conversation header (no messages).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Conversation {
    pub id: String,
    /// The model that was active when the conversation was created.
    pub model_id: String,
    pub model_filename: String,
    /// Which backend was running: `"vulkan"`, `"cpu"`, or `""` if unknown.
    pub backend: String,
    /// Short human-readable title (first user message, trimmed).
    pub title: String,
    /// Unix timestamp (seconds) of creation.
    pub created_at: i64,
    /// Unix timestamp (seconds) of the most recent message.
    pub updated_at: i64,
}

/// A single message within a conversation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoredMessage {
    pub id: i64,
    pub conversation_id: String,
    /// `"user"`, `"assistant"`, or `"system"`.
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

/// Arguments for creating a new conversation.
#[derive(Deserialize, Debug)]
pub struct NewConversationArgs {
    pub model_id: String,
    pub model_filename: String,
    pub backend: String,
    /// Optional override — defaults to `"New chat"` if omitted.
    pub title: Option<String>,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Generates a simple unique ID: `conv_<unix_secs>_<random 6 hex chars>`.
fn new_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    now_secs().hash(&mut h);
    std::thread::current().id().hash(&mut h);
    format!("conv_{:x}_{:06x}", now_secs(), h.finish() & 0xFF_FFFF)
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Creates a new conversation and returns it.
///
/// The `id` is generated server-side so the frontend doesn't need uuid support.
#[tauri::command]
pub fn create_conversation(
    app: AppHandle,
    args: NewConversationArgs,
) -> Result<Conversation, String> {
    let conn  = open_db(&app)?;
    let now   = now_secs();
    let id    = new_id();
    let title = args.title.unwrap_or_else(|| "New chat".to_string());

    conn.execute(
        "INSERT INTO conversations
            (id, model_id, model_filename, backend, title, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, args.model_id, args.model_filename, args.backend, title, now, now],
    )
    .map_err(|e| format!("Could not create conversation: {e}"))?;

    Ok(Conversation {
        id,
        model_id: args.model_id,
        model_filename: args.model_filename,
        backend: args.backend,
        title,
        created_at: now,
        updated_at: now,
    })
}

/// Lists all conversations, most-recently-updated first.
#[tauri::command]
pub fn list_conversations(app: AppHandle) -> Result<Vec<Conversation>, String> {
    let conn = open_db(&app)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, model_id, model_filename, backend, title, created_at, updated_at
             FROM conversations
             ORDER BY updated_at DESC",
        )
        .map_err(|e| format!("Could not prepare list query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Conversation {
                id:             row.get(0)?,
                model_id:       row.get(1)?,
                model_filename: row.get(2)?,
                backend:        row.get(3)?,
                title:          row.get(4)?,
                created_at:     row.get(5)?,
                updated_at:     row.get(6)?,
            })
        })
        .map_err(|e| format!("Could not query conversations: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Could not read conversation row: {e}"))
}

/// Returns all messages in a conversation, in chronological order.
#[tauri::command]
pub fn get_conversation_messages(
    app: AppHandle,
    conversation_id: String,
) -> Result<Vec<StoredMessage>, String> {
    let conn = open_db(&app)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, conversation_id, role, content, created_at
             FROM messages
             WHERE conversation_id = ?1
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|e| format!("Could not prepare message query: {e}"))?;

    let rows = stmt
        .query_map(params![conversation_id], |row| {
            Ok(StoredMessage {
                id:              row.get(0)?,
                conversation_id: row.get(1)?,
                role:            row.get(2)?,
                content:         row.get(3)?,
                created_at:      row.get(4)?,
            })
        })
        .map_err(|e| format!("Could not query messages: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Could not read message row: {e}"))
}

/// Appends a message to an existing conversation and bumps `updated_at`.
///
/// Returns the newly inserted message (including its auto-generated `id`).
#[tauri::command]
pub fn append_message(
    app: AppHandle,
    conversation_id: String,
    role: String,
    content: String,
) -> Result<StoredMessage, String> {
    let conn = open_db(&app)?;
    let now  = now_secs();

    conn.execute(
        "INSERT INTO messages (conversation_id, role, content, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![conversation_id, role, content, now],
    )
    .map_err(|e| format!("Could not insert message: {e}"))?;

    let row_id = conn.last_insert_rowid();

    // Keep updated_at current so list_conversations sorts correctly.
    conn.execute(
        "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
        params![now, conversation_id],
    )
    .map_err(|e| format!("Could not bump updated_at: {e}"))?;

    Ok(StoredMessage {
        id: row_id,
        conversation_id,
        role,
        content,
        created_at: now,
    })
}

/// Renames a conversation's title.
#[tauri::command]
pub fn rename_conversation(
    app: AppHandle,
    conversation_id: String,
    title: String,
) -> Result<(), String> {
    let conn = open_db(&app)?;
    conn.execute(
        "UPDATE conversations SET title = ?1 WHERE id = ?2",
        params![title, conversation_id],
    )
    .map_err(|e| format!("Could not rename conversation: {e}"))?;
    Ok(())
}

/// Deletes a conversation and all its messages (via `ON DELETE CASCADE`).
#[tauri::command]
pub fn delete_conversation(
    app: AppHandle,
    conversation_id: String,
) -> Result<(), String> {
    let conn = open_db(&app)?;
    conn.execute(
        "DELETE FROM conversations WHERE id = ?1",
        params![conversation_id],
    )
    .map_err(|e| format!("Could not delete conversation: {e}"))?;
    Ok(())
}