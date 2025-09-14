use std::fs;
use color_eyre::Result;
use rust_test::sqlite::Db;

fn temp_dir() -> Result<tempfile::TempDir> {
    Ok(tempfile::tempdir()?)
}

#[test]
fn text_read_write_cycle() -> Result<()> {
    let mut db = Db::in_memory()?;
    db.upsert_text("notes/hello.txt", "こんにちは")?;
    assert_eq!(db.read_text("notes/hello.txt")?, "こんにちは");
    // overwrite
    db.upsert_text("notes/hello.txt", "再び")?;
    assert_eq!(db.read_text("notes/hello.txt")?, "再び");
    Ok(())
}

#[test]
fn bytes_read_write_cycle() -> Result<()> {
    let mut db = Db::in_memory()?;
    let data = vec![1u8, 2, 3, 4, 255];
    db.upsert_bytes("bin/data.bin", &data)?;
    assert_eq!(db.read_bytes("bin/data.bin")?, data);
    Ok(())
}

#[test]
fn list_and_delete() -> Result<()> {
    let mut db = Db::in_memory()?;
    db.upsert_text("dir/a.txt", "A")?;
    db.upsert_text("dir/b.txt", "B")?;
    db.upsert_text("dir/sub/c.txt", "C")?;
    let all = db.list_files("dir/%")?;
    assert_eq!(all.len(), 3);
    assert!(db.delete("dir/b.txt")?);
    assert!(!db.delete("dir/b.txt")?); // already deleted
    let remain = db.list_files("dir/%")?;
    assert_eq!(remain.len(), 2);
    Ok(())
}

#[test]
fn import_export_cycle() -> Result<()> {
    let tmp = temp_dir()?;
    let fs_file = tmp.path().join("sample.txt");
    fs::write(&fs_file, "Hello FS")?;

    let db_file = tmp.path().join("store.sqlite");
    let mut db = Db::open_or_create(&db_file)?;

    db.import_file_from_fs(&fs_file, "files/sample.txt")?;
    assert!(db.exists("files/sample.txt")?);
    assert_eq!(db.read_text("files/sample.txt")?, "Hello FS");

    let out_file = tmp.path().join("exported.txt");
    db.export_file_to_fs("files/sample.txt", &out_file)?;
    let exported = fs::read_to_string(&out_file)?;
    assert_eq!(exported, "Hello FS");

    Ok(())
}

#[test]
fn persistence_between_connections() -> Result<()> {
    let tmp = temp_dir()?;
    let db_path = tmp.path().join("persist.sqlite");
    {
    let mut db = Db::open_or_create(&db_path)?;
    db.upsert_text("k1", "v1")?;
    }
    {
        let db = Db::open_or_create(&db_path)?;
        assert_eq!(db.read_text("k1")?, "v1");
    }
    Ok(())
}
