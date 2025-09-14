//! SQLite ユーティリティモジュール
//!
//! `rusqlite` を使って、簡単な "仮想ファイルストレージ" を SQLite 上に実装します。
//! 主な目的: テキスト/バイナリデータをキー(`path`)で保存し、読み出し/列挙/削除/FS とのインポート・エクスポートを
//! シンプルな API で行えるようにすること。
//!
//! # 特色
//! - Windows でもビルドしやすいように `bundled` フィーチャを利用
//! - UPSERT (`INSERT .. ON CONFLICT`) によりシンプルに更新
//! - テキストとバイナリを統一して BLOB カラムに保存 (テキストは UTF-8 として格納)
//! - `modified_at` で更新時刻を保持
//!
//! # 代表的な使い方
//! ```no_run
//! use rust_test::sqlite::{Db, FileEntry};
//!
//! # fn demo() -> color_eyre::Result<()> {
//! let db = Db::open_or_create("app_data.sqlite")?; // ファイルが無ければ作成
//! db.upsert_text("notes/hello.txt", "こんにちは")?;
//! let content = db.read_text("notes/hello.txt")?;
//! println!("{}", content); // => こんにちは
//! for entry in db.list_files("notes/%")? { // LIKE パターン
//!     println!("{} ({} bytes)", entry.path, entry.size_bytes);
//! }
//! # Ok(()) }
//! ```
//!
//! # テスト
//! `tests/sqlite_util_tests.rs` を参照。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::eyre::{eyre, Result};
use rusqlite::{params, Connection, OptionalExtension};
use tracing::{debug, info};

/// DB ハンドル。内部で `rusqlite::Connection` を保持します。
pub struct Db {
	conn: Connection,
	path: PathBuf,
}

/// `files` テーブルに対応するメタ情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
	pub path: String,
	pub size_bytes: i64,
	pub modified_at_epoch_ms: i64,
}

impl Db {
	/// DB を開き、存在しなければ新規作成。
	pub fn open_or_create<P: AsRef<Path>>(path: P) -> Result<Self> {
		let p = path.as_ref().to_path_buf();
		let conn = Connection::open(&p)?;
		let db = Self { conn, path: p };
		db.ensure_schema()?;
		Ok(db)
	}

	/// メモリ上 (永続化なし) の DB を作成 (主にテスト用途)
	pub fn in_memory() -> Result<Self> {
		let conn = Connection::open_in_memory()?;
		let db = Self { conn, path: PathBuf::from(":memory:") };
		db.ensure_schema()?;
		Ok(db)
	}

	/// スキーマを作成 (存在しない場合のみ)
	fn ensure_schema(&self) -> Result<()> {
		self.conn.execute_batch(
			r#"
			CREATE TABLE IF NOT EXISTS files (
				path TEXT PRIMARY KEY,
				data BLOB NOT NULL,
				size_bytes INTEGER NOT NULL,
				modified_at_epoch_ms INTEGER NOT NULL
			);
			CREATE INDEX IF NOT EXISTS idx_files_modified ON files(modified_at_epoch_ms DESC);
			"#,
		)?;
		Ok(())
	}

	/// 現在時刻 (ms since epoch)
	fn now_ms() -> Result<i64> {
		Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64)
	}

	/// テキストを UTF-8 として保存 (path が既に存在すれば更新)
	pub fn upsert_text<P: AsRef<str>>(&mut self, path: P, text: &str) -> Result<()> {
		self.upsert_bytes(path, text.as_bytes())
	}

	/// 任意のバイト列を保存 (path が既に存在すれば更新)
	pub fn upsert_bytes<P: AsRef<str>>(&mut self, path: P, data: &[u8]) -> Result<()> {
		let path_ref = path.as_ref();
		let modified = Self::now_ms()?;
		let size = data.len() as i64;
		self.conn.execute(
			r#"INSERT INTO files(path, data, size_bytes, modified_at_epoch_ms)
			   VALUES (?1, ?2, ?3, ?4)
			   ON CONFLICT(path) DO UPDATE SET
				   data = excluded.data,
				   size_bytes = excluded.size_bytes,
				   modified_at_epoch_ms = excluded.modified_at_epoch_ms"#,
			params![path_ref, data, size, modified],
		)?;
		debug!(target: "sqlite", "upsert path={}", path_ref);
		Ok(())
	}

	/// テキスト (UTF-8) を読み出し
	pub fn read_text<P: AsRef<str>>(&self, path: P) -> Result<String> {
		let bytes = self.read_bytes(path.as_ref())?;
		Ok(String::from_utf8(bytes)?)
	}

	/// バイト列を読み出し
	pub fn read_bytes<P: AsRef<str>>(&self, path: P) -> Result<Vec<u8>> {
		let mut stmt = self.conn.prepare("SELECT data FROM files WHERE path = ?1")?;
		let maybe: Option<Vec<u8>> = stmt
			.query_row(params![path.as_ref()], |row| row.get(0))
			.optional()?;
		maybe.ok_or_else(|| eyre!("path not found: {}", path.as_ref()))
	}

	/// 削除 (存在しなくても OK) 戻り値: 削除したか
	pub fn delete<P: AsRef<str>>(&self, path: P) -> Result<bool> {
		let affected = self.conn.execute("DELETE FROM files WHERE path = ?1", params![path.as_ref()])?;
		Ok(affected > 0)
	}

	/// LIKE パターン (例: "notes/%") でファイル一覧を取得。`%` や `_` を含めないで完全一致一覧を得たい場合はそのままパスを渡せばよい。
	pub fn list_files<P: AsRef<str>>(&self, like_pattern: P) -> Result<Vec<FileEntry>> {
		let mut stmt = self.conn.prepare(
			"SELECT path, size_bytes, modified_at_epoch_ms FROM files WHERE path LIKE ?1 ORDER BY path ASC",
		)?;
		let iter = stmt.query_map(params![like_pattern.as_ref()], |row| {
			Ok(FileEntry {
				path: row.get(0)?,
				size_bytes: row.get(1)?,
				modified_at_epoch_ms: row.get(2)?,
			})
		})?;
		let mut out = Vec::new();
		for r in iter { out.push(r?); }
		Ok(out)
	}

	/// OS のファイルを読み込み DB に保存 (既に存在する場合は上書き)
	pub fn import_file_from_fs<P: AsRef<Path>, Q: AsRef<str>>(&mut self, fs_path: P, db_path: Q) -> Result<()> {
		let data = fs::read(&fs_path)?;
		let db_path_str = db_path.as_ref().to_string();
		self.upsert_bytes(&db_path_str, &data)?;
		info!(target: "sqlite", "imported {:?} -> {} ({} bytes)", fs_path.as_ref(), db_path_str, data.len());
		Ok(())
	}

	/// DB の内容を OS 上のファイルへ書き出し (親ディレクトリが無ければ作成)
	pub fn export_file_to_fs<P: AsRef<str>, Q: AsRef<Path>>(&self, db_path: P, fs_path: Q) -> Result<()> {
		let data = self.read_bytes(db_path.as_ref())?;
		if let Some(parent) = fs_path.as_ref().parent() { fs::create_dir_all(parent)?; }
		fs::write(&fs_path, &data)?;
		info!(target: "sqlite", "exported {} -> {:?} ({} bytes)", db_path.as_ref(), fs_path.as_ref(), data.len());
		Ok(())
	}

	/// DB ファイルのパス
	pub fn db_file_path(&self) -> &Path { &self.path }
}

// 追加の拡張 (必要に応じて)
impl Db {
	/// ファイルの存在確認
	pub fn exists<P: AsRef<str>>(&self, path: P) -> Result<bool> {
		let mut stmt = self.conn.prepare("SELECT 1 FROM files WHERE path=?1 LIMIT 1")?;
		let val: Option<i64> = stmt.query_row(params![path.as_ref()], |row| row.get(0)).optional()?;
		Ok(val.is_some())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_text_cycle() -> Result<()> {
		let mut db = Db::in_memory()?;
		db.upsert_text("a.txt", "hello")?;
		assert_eq!(db.read_text("a.txt")?, "hello");
		db.upsert_text("a.txt", "world")?;
		assert_eq!(db.read_text("a.txt")?, "world");
		Ok(())
	}
}

