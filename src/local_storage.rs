use anyhow::Result;
use runner::local_storage::sql_ops::*;
use rusqlite::{types, Connection};
use std::path::Path;
use wasmtime::component::*;

bindgen!();

struct SQLStorage {
    conn: Connection,
}

impl SQLStorage {
    pub fn new<P>(p: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            conn: Connection::open(p).unwrap(),
        }
    }
}

impl From<types::Value> for DataType {
    fn from(value: types::Value) -> Self {
        unreachable!()
    }
}

impl Host for SQLStorage {
    fn query(&mut self, q: String) -> Result<Vec<Row>> {
        let mut stmt = self.conn.prepare(&q).unwrap();
        let column_names: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
        let rows: Vec<Row> = stmt.query_and_then(|row| {
            let mut r: Vec<Entry> = Vec::new();
            for (i, col_name) in column_names.iter().enumerate() {
                let value: = row.get(i).unwrap();
                r.push(Entry {
                    field_name: col_name.to_string(),
                    value: value.into(),
                })
            }
            Ok(r)
        });
        Ok(rows)
    }
}
