pub(crate) const DB: &str = "user_info.db";

#[derive(Debug, Clone)]
pub(crate) struct UserInfo {
    pub(crate) id: i64,
    pub(crate) user_id: i64,
    pub(crate) somm_address: String,
}

pub(crate) fn init(db: &str) -> Result<(), rusqlite::Error> {
    let conn = rusqlite::Connection::open(db)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_info (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL UNIQUE,
            somm_address TEXT NOT NULL UNIQUE
        )",
        [],
    )?;

    Ok(())
}

pub(crate) fn get_connection() -> Result<rusqlite::Connection, rusqlite::Error> {
    connect(DB)
}

pub fn connect(db: &str) -> Result<rusqlite::Connection, rusqlite::Error> {
    rusqlite::Connection::open(db)
}

pub(crate) fn get_user_info(conn: &rusqlite::Connection, user_id: i64) -> Result<Option<UserInfo>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT * FROM user_info WHERE user_id = ?")?;
    let mut rows = stmt.query([user_id])?;

    if let Some(row) = rows.next()? {
        let user = UserInfo {
            id: row.get(0)?,
            user_id: row.get(1)?,
            somm_address: row.get(2)?,
        };

        Ok(Some(user))
    } else {
        Ok(None)
    }
}

pub(crate) fn insert_user_info(conn: &rusqlite::Connection, user_id: i64, somm_address: &str) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "INSERT INTO user_info (user_id, somm_address) VALUES (?, ?)",
        [user_id.to_string(), somm_address.to_string()],
    )
}

pub(crate) fn update_user_info(conn: &rusqlite::Connection, user_id: i64, somm_address: &str) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "UPDATE user_info SET somm_address = ? WHERE user_id = ?",
        [somm_address.to_string(), user_id.to_string()],
    )
}

pub(crate) fn delete_user_info(conn: &rusqlite::Connection, user_id: i64) -> Result<usize, rusqlite::Error> {
    conn.execute("DELETE FROM user_info WHERE user_id = ?", [user_id])
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DBFixture {
        db: String,
    }

    impl DBFixture {
        fn init(db: &str) -> Self {
            init(db).expect("error while initializing database");

            Self {
                db: db.to_owned(),
            }
        }

        fn connect(&self) -> rusqlite::Connection {
            connect(self.db.as_str()).expect("error while connecting to database")
        }

        fn insert_user_info(&self, conn: &rusqlite::Connection, user_id: i64, somm_address: &str) -> usize {
            insert_user_info(conn, user_id, somm_address).expect("error while inserting user info")
        }

        fn get_user_info(&self, conn: &rusqlite::Connection, user_id: i64) -> Option<UserInfo> {
            get_user_info(conn, user_id).expect("unexpected error while getting user info")
        }

        fn update_user_info(&self, conn: &rusqlite::Connection, user_id: i64, somm_address: &str) -> usize {
            update_user_info(conn, user_id, somm_address).expect("error while updating user info")
        }

        fn delete_user_info(&self, conn: &rusqlite::Connection, user_id: i64) -> usize {
            delete_user_info(conn, user_id).expect("error while deleting user info")
        }
    }

    impl Drop for DBFixture {
        #[allow(unused_must_use)]
        fn drop(&mut self) {
            let db = &self.db;
            std::fs::remove_file(db);
        }
    }

    #[test]
    fn test_init() {
        let db = "test_init";
        let _fixture = DBFixture::init(db);
        DBFixture::init(db);
    }

    #[test]
    fn test_connect() {
        let db = "test_connect";
        let fixture = DBFixture::init(db);
        
        fixture.connect();
    }

    #[test]
    fn test_insert_user_info() {
        let db = "test_insert_user_info";
        let fixture = DBFixture::init(db);
        let conn = fixture.connect();
        assert_eq!(1, fixture.insert_user_info(&conn, 1, "somm_address"), "insert did not result in 1 row change");
    }

    #[test]
    fn test_get_user_info() {
        let db = "test_get_user_info";
        let fixture = DBFixture::init(db);
        let conn = fixture.connect();
        assert_eq!(1, fixture.insert_user_info(&conn, 1, "somm_address"), "insert did not result in 1 row change");
        assert!(fixture.get_user_info(&conn, 1).is_some(), "no user info found"); 
    }

    #[test]
    fn test_update_user_info() {
        let db = "test_update_user_info";
        let fixture = DBFixture::init(db);
        let conn = fixture.connect();
        assert_eq!(1, fixture.insert_user_info(&conn, 1, "somm_address"), "insert did not result in 1 row change");
        assert_eq!(1, fixture.update_user_info(&conn, 1, "new_somm_address"), "update did not result in 1 row change");    
    }

    #[test]
    fn test_delete_user_info() {
        let db = "test_delete_user_info";
        let fixture = DBFixture::init(db);
        let conn = fixture.connect();
        assert_eq!(1, fixture.insert_user_info(&conn, 1, "somm_address"), "insert did not result in 1 row change");
        assert_eq!(1, fixture.delete_user_info(&conn, 1), "delete did not result in 1 row change");
    }
}
