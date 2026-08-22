import Database from 'better-sqlite3';
import type { StorageAdapter } from 'grammy';
import { mkdirSync } from 'fs';
import path from 'path';

const dbPath = process.env.DB_PATH || 'data/faucet.db';
const dbDir = path.dirname(dbPath);
mkdirSync(dbDir, { recursive: true });

const db = new Database(dbPath);
db.exec(`
  CREATE TABLE IF NOT EXISTS user_calls (
    user_id INTEGER PRIMARY KEY,
    last_call TEXT NOT NULL
  )
`);
db.exec(`
  CREATE TABLE IF NOT EXISTS sessions (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
  )
`);
db.pragma('journal_mode = WAL');

export function getLastCall(userId: number): Date | null {
  const row = db.prepare('SELECT last_call FROM user_calls WHERE user_id = ?').get(userId) as
    | { last_call: string }
    | undefined;
  return row ? new Date(row.last_call) : null;
}

export function setLastCall(userId: number, date: Date): void {
  db.prepare('INSERT OR REPLACE INTO user_calls (user_id, last_call) VALUES (?, ?)').run(
    userId,
    date.toISOString(),
  );
}

interface SessionRow {
  value: string;
}

export function sqliteSessionStorage<T>(): StorageAdapter<T> {
  const readStmt = db.prepare('SELECT value FROM sessions WHERE key = ?');
  const writeStmt = db.prepare('INSERT OR REPLACE INTO sessions (key, value) VALUES (?, ?)');
  const deleteStmt = db.prepare('DELETE FROM sessions WHERE key = ?');

  return {
    read: async (key: string) => {
      const row = readStmt.get(key) as SessionRow | undefined;
      return row ? (JSON.parse(row.value) as T) : undefined;
    },
    write: async (key: string, value: T) => {
      writeStmt.run(key, JSON.stringify(value));
    },
    delete: async (key: string) => {
      deleteStmt.run(key);
    },
  };
}
