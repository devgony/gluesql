#![deny(clippy::str_to_string)]

use gluesql_core::{
    chrono::Utc,
    store::{GlueObjects, Meta, MetaName, MetaRow, Metadata},
};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStorage {
    pub id_counter: i64,
    pub items: HashMap<String, Item>,
    pub metadata: HashMap<MetaName, Item>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        let schema = GlueObjects::to_schema();

        let rows = IndexMap::default();

        let glue_objects = Item { schema, rows };

        Self {
            id_counter: 0,
            items: HashMap::new(),
            metadata: HashMap::from([(MetaName::GlueObjects, glue_objects)]),
        }
    }
}

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
    async fn scan_meta(&self, meta: &MetaName) -> Result<RowIter> {
        let rows: RowIter = match self.metadata.get(meta) {
            Some(item) => Box::new(item.rows.clone().into_iter().map(Ok)),
            None => Box::new(empty()),
        };

        Ok(rows)
    }

    async fn append_meta(&mut self, meta: Meta) -> Result<()> {
        if let Some(item) = self.metadata.get_mut(&meta.name) {
            self.id_counter += 1;
            let row = meta.row.to_values();

            item.rows.insert(Key::I64(self.id_counter), row);
        }

        Ok(())
    }

    async fn insert_meta(&mut self, meta: &Meta, rows: Vec<(Key, DataRow)>) -> Result<()> {
        todo!();
        // if let Some(item) = self.metadata.get_mut(&meta.name) {
        //     for (key, row) in rows {
        //         item.rows.insert(key, row);
        //     }
        // }

        // Ok(())
    }

    async fn delete_meta(&mut self, meta: &Meta, keys: Vec<Key>) -> Result<()> {
        if let Some(item) = self.metadata.get_mut(&meta.name) {
            for key in keys {
                item.rows.remove(&key);
            }
        }

        Ok(())
    }
}

#[async_trait(?Send)]
impl StoreMut for MemoryStorage {
    async fn insert_schema(&mut self, schema: &Schema) -> Result<()> {
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
    use gluesql_core::store::{MetaName, Metadata};

    use crate::MemoryStorage;

    #[test]
    fn scan_meta_test() {
        use futures::executor::block_on;
        use gluesql_core::prelude::Glue;

        let storage = MemoryStorage::default();
        let mut glue = Glue::new(storage);

        glue.execute("CREATE TABLE MetaTest").unwrap();
        block_on(async {
            let a = glue
                .storage
                .scan_meta(&MetaName::GlueObjects)
                .await
                .unwrap();

            for b in a {
                println!("{b:#?}");
            }
        });

        assert!(false);
    }
}
