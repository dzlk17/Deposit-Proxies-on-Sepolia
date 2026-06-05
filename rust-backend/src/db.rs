use rusqlite::{params, Connection, Result};

use crate::models::DepositRecord;

pub fn init_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS deposits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_address TEXT NOT NULL,
            deposit_address TEXT NOT NULL UNIQUE,
            salt TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )?;
    Ok(conn)
}

pub fn insert_deposit(conn: &Connection, record: &DepositRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO deposits (user_address, deposit_address, salt, status)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            record.user_address,
            record.deposit_address,
            record.salt,
            record.status
        ],
    )?;
    Ok(())
}

pub fn get_all_deposits(conn: &Connection) -> Result<Vec<DepositRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_address, deposit_address, salt, status, created_at FROM deposits ORDER BY id",
    )?;
    let records = stmt
        .query_map([], |row| {
            Ok(DepositRecord {
                id: row.get(0)?,
                user_address: row.get(1)?,
                deposit_address: row.get(2)?,
                salt: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(records)
}

pub fn update_status(conn: &Connection, deposit_address: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE deposits SET status = ?1 WHERE deposit_address = ?2",
        params![status, deposit_address],
    )?;
    Ok(())
}
