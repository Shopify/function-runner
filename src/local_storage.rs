use anyhow::{Result, Error};
use runner::local_storage::sql_ops::*;
use rusqlite::{types, Connection};
use std::path::Path;
use wasmtime::component::*;

bindgen!();

pub struct SQLStorage {
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
        match value {
            types::Value::Null => DataType::Null,
            types::Value::Integer(i) => DataType::Int64(i),
            types::Value::Real(r) => DataType::Float(r),
            types::Value::Text(t) => DataType::Str(t),
            types::Value::Blob(b) => DataType::Binary(b),
        }
    }
}

impl Host for SQLStorage {
    fn query(&mut self, q: String) -> Result<Vec<Row>> {
        let mut stmt = self.conn.prepare(&q)?;
        let column_names: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
        let rows_result = stmt.query_and_then([], |row| {
            let mut r: Vec<Entry> = Vec::new();
            for (i, col_name) in column_names.iter().enumerate() {
                let value: types::Value = row.get(i)?;
                r.push(Entry {
                    field_name: col_name.to_string(),
                    value: value.into(),
                })
            }
            Ok(r)
        });

        // Handle the Result and collect the rows into a Vec
        let rows: Vec<Row> = rows_result?
        .map(|r: Result<Vec<Entry>, rusqlite::Error>| r.map_err(Error::from))
        .collect::<Result<Vec<Row>, Error>>()?;

        Ok(rows)
    }
}
