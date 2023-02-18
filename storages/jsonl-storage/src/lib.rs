mod alter_table;
mod error;
mod index;
mod transaction;

use {
    async_trait::async_trait,
    error::{JsonlStorageError, OptionExt, ResultExt},
    gluesql_core::{
        data::{HashMapJsonExt, Schema},
        prelude::Key,
        result::Result,
        store::{DataRow, RowIter, Store},
        {chrono::NaiveDateTime, store::StoreMut},
    },
    serde_json::{Map, Value as JsonValue},
    std::{
        cmp::Ordering,
        collections::HashMap,
        fs::{self, remove_file, File, OpenOptions},
        io::{self, BufRead, Read, Write},
        iter::Peekable,
        path::{Path, PathBuf},
        vec::IntoIter,
    },
};

#[derive(Debug)]
pub struct JsonlStorage {
    pub path: PathBuf,
}

impl JsonlStorage {
    pub fn new(path: &str) -> Result<Self> {
        fs::create_dir_all(path).map_storage_err()?;
        let path = PathBuf::from(path);

        Ok(Self { path })
    }

    fn fetch_schema(&self, table_name: &str) -> Result<Option<Schema>> {
        if !self.data_path(table_name).exists() {
            return Ok(None);
        };

        let schema_path = self.schema_path(table_name);
        let column_defs = match schema_path.exists() {
            true => {
                let mut file = File::open(&schema_path).map_storage_err()?;
                let mut ddl = String::new();
                file.read_to_string(&mut ddl).map_storage_err()?;

                Schema::from_ddl(&ddl).map(|schema| schema.column_defs)
            }
            false => Ok(None),
        }?;

        Ok(Some(Schema {
            table_name: table_name.to_owned(),
            column_defs,
            indexes: vec![],
            created: NaiveDateTime::default(),
            engine: None,
        }))
    }

    fn data_path(&self, table_name: &str) -> PathBuf {
        let path = self.path_by(table_name, "jsonl");

        PathBuf::from(path)
    }

    fn schema_path(&self, table_name: &str) -> PathBuf {
        let path = self.path_by(table_name, "sql");

        PathBuf::from(path)
    }

    fn path_by(&self, table_name: &str, extension: &str) -> String {
        let path = format!("{}/{}.{extension}", self.path.display(), table_name);

        path
    }

    fn scan_data(&self, table_name: &str) -> Result<RowIter> {
        let schema = self
            .fetch_schema(table_name)?
            .map_storage_err(JsonlStorageError::TableDoesNotExist.to_string())?;
        let data_path = self.data_path(table_name);
        let lines = read_lines(data_path).map_storage_err()?;
        let row_iter = lines.enumerate().map(move |(key, line)| -> Result<_> {
            let hash_map = HashMap::parse_json_object(&line.map_storage_err()?)?;
            let data_row = match &schema.column_defs {
                Some(column_defs) => {
                    let values = column_defs
                        .iter()
                        .map(|column_def| -> Result<_> {
                            let value = hash_map
                                .get(&column_def.name)
                                .map_storage_err(JsonlStorageError::ColumnDoesNotExist.to_string())?
                                .clone();
                            let data_type = value.get_type();
                            match data_type {
                                Some(data_type) => match data_type == column_def.data_type {
                                    true => Ok(value),
                                    false => value.cast(&column_def.data_type),
                                },
                                None => Ok(value),
                            }
                        })
                        .collect::<Result<Vec<_>>>()?;

                    DataRow::Vec(values)
                }
                None => DataRow::Map(hash_map),
            };
            let key = Key::I64((key + 1).try_into().map_storage_err()?);

            Ok((key, data_row))
        });

        Ok(Box::new(row_iter))
    }
}

#[async_trait(?Send)]
impl Store for JsonlStorage {
    async fn fetch_schema(&self, table_name: &str) -> Result<Option<Schema>> {
        self.fetch_schema(table_name)
    }

    async fn fetch_all_schemas(&self) -> Result<Vec<Schema>> {
        let paths = fs::read_dir(&self.path).map_storage_err()?;
        let mut schemas = paths
            .filter(|result| {
                result
                    .as_ref()
                    .map(|dir_entry| {
                        dir_entry
                            .path()
                            .extension()
                            .map(|os_str| os_str.to_str() == Some("jsonl"))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            })
            .map(|result| -> Result<_> {
                let path = result.map_storage_err()?.path();
                let table_name = path
                    .file_stem()
                    .map_storage_err(JsonlStorageError::FileNotFound.to_string())?
                    .to_str()
                    .map_storage_err(JsonlStorageError::FileNotFound.to_string())?
                    .to_owned();

                self.fetch_schema(table_name.as_str())?
                    .map_storage_err(JsonlStorageError::TableDoesNotExist.to_string())
            })
            .collect::<Result<Vec<Schema>>>()?;

        schemas.sort_by(|a, b| a.table_name.cmp(&b.table_name));

        Ok(schemas)
    }

    async fn fetch_data(&self, table_name: &str, target: &Key) -> Result<Option<DataRow>> {
        let row = self.scan_data(table_name)?.find_map(|result| {
            result
                .map(|(key, row)| (&key == target).then_some(row))
                .unwrap_or(None)
        });

        Ok(row)
    }

    async fn scan_data(&self, table_name: &str) -> Result<RowIter> {
        self.scan_data(table_name)
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

struct SortMerge<T: Iterator<Item = Result<(Key, DataRow)>>> {
    left_rows: Peekable<T>,
    right_rows: Peekable<IntoIter<(Key, DataRow)>>,
}

impl<T> SortMerge<T>
where
    T: Iterator<Item = Result<(Key, DataRow)>>,
{
    fn new(left_rows: T, right_rows: IntoIter<(Key, DataRow)>) -> Self {
        let left_rows = left_rows.peekable();
        let right_rows = right_rows.peekable();

        Self {
            left_rows,
            right_rows,
        }
    }
}
impl<T> Iterator for SortMerge<T>
where
    T: Iterator<Item = Result<(Key, DataRow)>>,
{
    type Item = Result<DataRow>;

    fn next(&mut self) -> Option<Self::Item> {
        let left = self.left_rows.peek();
        let right = self.right_rows.peek();

        let (left_key, right_key) = match (left, right) {
            (Some(Ok((left_key, _))), Some((right_key, _))) => (left_key, right_key),
            (Some(_), _) => {
                return self.left_rows.next().map(|item| Ok(item?.1));
            }
            (None, Some(_)) => {
                return self.right_rows.next().map(|item| item.1).map(Ok);
            }
            (None, None) => {
                return None;
            }
        };

        match left_key.to_cmp_be_bytes().cmp(&right_key.to_cmp_be_bytes()) {
            Ordering::Less => self.left_rows.next(),
            Ordering::Greater => self.right_rows.next().map(Ok),
            Ordering::Equal => {
                self.left_rows.next();
                self.right_rows.next().map(Ok)
            }
        }
        .map(|item| Ok(item?.1))
    }
}

#[async_trait(?Send)]
impl StoreMut for JsonlStorage {
    async fn insert_schema(&mut self, schema: &Schema) -> Result<()> {
        let data_path = self.data_path(schema.table_name.as_str());
        File::create(data_path).map_storage_err()?;

        if schema.column_defs.is_some() {
            let schema_path = self.schema_path(schema.table_name.as_str());
            let ddl = schema.to_ddl();
            let mut file = File::create(schema_path).map_storage_err()?;
            write!(file, "{ddl}").map_storage_err()?;
        }

        Ok(())
    }

    async fn delete_schema(&mut self, table_name: &str) -> Result<()> {
        let data_path = self.data_path(table_name);
        if data_path.exists() {
            remove_file(data_path).map_storage_err()?;
        }

        let schema_path = self.schema_path(table_name);
        if schema_path.exists() {
            remove_file(schema_path).map_storage_err()?;
        }

        Ok(())
    }

    async fn append_data(&mut self, table_name: &str, rows: Vec<DataRow>) -> Result<()> {
        let schema = self
            .fetch_schema(table_name)?
            .map_storage_err(JsonlStorageError::TableDoesNotExist.to_string())?;
        let table_path = JsonlStorage::data_path(self, table_name);

        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(table_path)
            .map_storage_err()?;

        let column_defs = schema.column_defs.unwrap_or_default();
        let labels = column_defs
            .iter()
            .map(|column_def| column_def.name.as_str())
            .collect::<Vec<_>>();

        for row in rows {
            let json_string = match row {
                DataRow::Vec(values) => {
                    let mut json_map = Map::new();
                    for (key, value) in labels.iter().zip(values.into_iter()) {
                        json_map.insert(key.to_string(), value.try_into()?);
                    }

                    JsonValue::Object(json_map).to_string()
                }
                DataRow::Map(hash_map) => {
                    let mut json_map = Map::new();
                    for (key, value) in hash_map {
                        json_map.insert(key.to_string(), value.try_into()?);
                    }

                    JsonValue::Object(json_map).to_string()
                }
            };
            writeln!(file, "{json_string}").map_storage_err()?;
        }

        Ok(())
    }

    async fn insert_data(&mut self, table_name: &str, mut rows: Vec<(Key, DataRow)>) -> Result<()> {
        let prev_rows = self.scan_data(table_name)?;
        rows.sort_by(|(key_a, _), (key_b, _)| {
            key_a.to_cmp_be_bytes().cmp(&key_b.to_cmp_be_bytes())
        });
        let rows = rows.into_iter();

        let sort_merge = SortMerge::new(prev_rows, rows);
        let merged = sort_merge.collect::<Result<Vec<_>>>()?;

        let table_path = self.data_path(table_name);
        File::create(&table_path).map_storage_err()?;

        self.append_data(table_name, merged).await
    }

    async fn delete_data(&mut self, table_name: &str, keys: Vec<Key>) -> Result<()> {
        let prev_rows = self.scan_data(table_name)?;
        let rows = prev_rows
            .filter_map(|result| {
                result
                    .map(|(key, data_row)| {
                        let preservable = !keys.iter().any(|target_key| target_key == &key);

                        preservable.then_some(data_row)
                    })
                    .unwrap_or(None)
            })
            .collect::<Vec<_>>();

        let table_path = self.data_path(table_name);
        File::create(&table_path).map_storage_err()?;

        self.append_data(table_name, rows).await
    }
}

#[test]
fn jsonl_storage_test() {
    use {
        crate::*,
        gluesql_core::{
            data::{SchemaParseError, ValueError},
            prelude::{
                Glue, {Payload, Value},
            },
            result::Error,
        },
    };

    let path = "./samples/";
    let jsonl_storage = JsonlStorage::new(path).unwrap();
    let mut glue = Glue::new(jsonl_storage);

    let actual = glue.execute("SELECT * FROM Schemaless").unwrap();
    let actual = actual.get(0).unwrap();
    let expected = Payload::SelectMap(vec![
        [("id".to_owned(), Value::I64(1))].into_iter().collect(),
        [("name".to_owned(), Value::Str("Glue".to_owned()))]
            .into_iter()
            .collect(),
        [
            ("id".to_owned(), Value::I64(3)),
            ("name".to_owned(), Value::Str("SQL".to_owned())),
        ]
        .into_iter()
        .collect(),
    ]);
    assert_eq!(actual, &expected);

    let actual = glue.execute("SELECT * FROM Schema").unwrap();
    let actual = actual.get(0).unwrap();
    let expected = Payload::Select {
        labels: ["id", "name"].into_iter().map(ToOwned::to_owned).collect(),
        rows: vec![
            vec![Value::I64(1), Value::Str("Glue".to_owned())],
            vec![Value::I64(2), Value::Str("SQL".to_owned())],
        ],
    };
    assert_eq!(actual, &expected);

    let actual = glue.execute("SELECT * FROM WrongFormat");
    let expected = Err(ValueError::InvalidJsonString("{".to_owned()).into());

    assert_eq!(actual, expected);

    let actual = glue.execute("SELECT * FROM WrongSchema");
    let expected = Err(Error::Schema(SchemaParseError::CannotParseDDL));

    assert_eq!(actual, expected);
}
