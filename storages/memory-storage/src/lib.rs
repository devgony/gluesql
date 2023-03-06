#![deny(clippy::str_to_string)]

use gluesql_core::{chrono::Utc, prelude::Value, store::Metadata};

mod alter_table;
mod index;
mod transaction;

use {
    async_trait::async_trait,
    gluesql_core::{
        data::{Key, Schema},
        result::Result,
        store::{DataRow, RowIter, Store, StoreMut},
    },
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, HashMap},
        iter::empty,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub schema: Schema,
    pub rows: BTreeMap<Key, DataRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryStorage {
    pub id_counter: i64,
    pub items: HashMap<String, Item>,
    pub metadata: HashMap<String, Value>,
}

/*
table_name metadata */

// impl Default for MemoryStorage {
//     fn default() -> Self {
//         let schema = GlueObjects::to_schema();

//         let rows = IndexMap::default();

//         let glue_objects = Item { schema, rows };

//         Self {
//             id_counter: 0,
//             items: HashMap::new(),
//             metadata: HashMap::from([(MetaName::GlueObjects, glue_objects)]),
//         }
//     }
// }

#[async_trait(?Send)]
impl Store for MemoryStorage {
    async fn fetch_all_schemas(&self) -> Result<Vec<Schema>> {
        let mut schemas = self
            .items
            .values()
            .map(|item| item.schema.clone())
            .collect::<Vec<_>>();
        schemas.sort_by(|a, b| a.table_name.cmp(&b.table_name));

        Ok(schemas)
    }
    async fn fetch_schema(&self, table_name: &str) -> Result<Option<Schema>> {
        self.items
            .get(table_name)
            .map(|item| Ok(item.schema.clone()))
            .transpose()
    }

    async fn fetch_data(&self, table_name: &str, key: &Key) -> Result<Option<DataRow>> {
        let row = self
            .items
            .get(table_name)
            .and_then(|item| item.rows.get(key).map(Clone::clone));

        Ok(row)
    }

    async fn scan_data(&self, table_name: &str) -> Result<RowIter> {
        let rows: RowIter = match self.items.get(table_name) {
            Some(item) => Box::new(item.rows.clone().into_iter().map(Ok)),
            None => Box::new(empty()),
        };

        Ok(rows)
    }
}

#[async_trait(?Send)]
impl Metadata for MemoryStorage {
    async fn scan_meta(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }

    async fn append_meta(&mut self, meta: HashMap<String, Value>) -> Result<()> {
        self.metadata.extend(meta);

        Ok(())
    }

    async fn delete_meta(&mut self, meta_name: &str) -> Result<()> {
        self.metadata.remove(meta_name);

        Ok(())
    }
}

#[async_trait(?Send)]
impl StoreMut for MemoryStorage {
    async fn insert_schema(&mut self, schema: &Schema) -> Result<()> {
        let created = Value::Map(HashMap::from([
            ("OBJECT_TYPE".to_owned(), Value::Str("TABLE".to_owned())),
            (
                "CREATED".to_owned(),
                Value::Timestamp(Utc::now().naive_utc()),
            ),
        ]));
        let meta = HashMap::from([(schema.table_name.clone(), created)]);
        self.append_meta(meta).await?;

        let table_name = schema.table_name.clone();
        let item = Item {
            schema: schema.clone(),
            rows: BTreeMap::new(),
        };
        self.items.insert(table_name, item);

        Ok(())
    }

    async fn delete_schema(&mut self, table_name: &str) -> Result<()> {
        self.items.remove(table_name);
        self.delete_meta(table_name).await?;

        Ok(())
    }

    async fn append_data(&mut self, table_name: &str, rows: Vec<DataRow>) -> Result<()> {
        if let Some(item) = self.items.get_mut(table_name) {
            for row in rows {
                self.id_counter += 1;

                item.rows.insert(Key::I64(self.id_counter), row);
            }
        }

        Ok(())
    }

    async fn insert_data(&mut self, table_name: &str, rows: Vec<(Key, DataRow)>) -> Result<()> {
        if let Some(item) = self.items.get_mut(table_name) {
            for (key, row) in rows {
                item.rows.insert(key, row);
            }
        }

        Ok(())
    }

    async fn delete_data(&mut self, table_name: &str, keys: Vec<Key>) -> Result<()> {
        if let Some(item) = self.items.get_mut(table_name) {
            for key in keys {
                item.rows.remove(&key);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::MemoryStorage,
        gluesql_core::prelude::{Payload, Value::*},
        test_suite::{row, select, stringify_label, test},
    };

    #[test]
    fn scan_meta_test() {
        use gluesql_core::prelude::Glue;

        let storage = MemoryStorage::default();
        let mut glue = Glue::new(storage);

        let cases = vec![
            (
                glue.execute("CREATE TABLE Meta (id INT, name TEXT)"),
                Ok(Payload::Create),
            ),
            (
                glue.execute(
                    "SELECT OBJECT_NAME, OBJECT_TYPE
                     FROM GLUE_OBJECTS
                     WHERE CREATED > NOW() - INTERVAL 1 MINUTE",
                ),
                Ok(select!(
                    OBJECT_NAME       | OBJECT_TYPE       ;
                    Str               | Str               ;
                    "Meta".to_owned()   "TABLE".to_owned()
                )),
            ),
            (glue.execute("DROP TABLE Meta"), Ok(Payload::DropTable)),
            (
                glue.execute(
                    "SELECT COUNT(*)
                     FROM GLUE_OBJECTS
                     WHERE CREATED > NOW() - INTERVAL 1 MINUTE",
                ),
                Ok(Payload::Select {
                    labels: vec!["COUNT(*)".to_owned()],
                    rows: Vec::new(),
                }),
            ),
        ];

        for (actual, expected) in cases {
            test(actual.map(|mut payloads| payloads.remove(0)), expected);
        }
    }
}
