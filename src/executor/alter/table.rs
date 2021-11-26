use crate::executor::execute;

use {
    super::{validate, AlterError},
    crate::{
        ast::{ColumnDef, ObjectName, Query, SetExpr, Statement, TableFactor},
        data::{get_name, Schema},
        result::MutResult,
        store::{GStore, GStoreMut},
    },
    futures::stream::{self, TryStreamExt},
    std::fmt::Debug,
};

pub async fn create_table<T: Debug, U: GStore<T> + GStoreMut<T>>(
    storage: U,
    name: &ObjectName,
    column_defs: &[ColumnDef],
    if_not_exists: bool,
    source: &Option<Box<Query>>,
) -> MutResult<U, ()> {
    match source {
        Some(v) => {
            if let SetExpr::Select(select_query) = &v.body {
                let TableFactor::Table {
                    name: source_name, ..
                } = &select_query.from.relation;
                let table_name = get_name(&source_name).unwrap();
                if let Some(Schema {
                    column_defs: source_column_defs,
                    ..
                }) = storage.fetch_schema(table_name).await.unwrap()
                {
                    let schema = (|| async {
                        let schema = Schema {
                            table_name: get_name(name).unwrap().to_string(),
                            column_defs: source_column_defs.clone(),
                            indexes: vec![],
                        };

                        for column_def in &schema.column_defs {
                            validate(column_def)?;
                        }

                        match (
                            storage.fetch_schema(&schema.table_name).await?,
                            if_not_exists,
                        ) {
                            (None, _) => Ok(Some(schema)),
                            (Some(_), true) => Ok(None),
                            (Some(_), false) => {
                                Err(AlterError::TableAlreadyExists(schema.table_name.to_owned())
                                    .into())
                            }
                        }
                    })()
                    .await;

                    let schema = match schema {
                        Ok(s) => s,
                        Err(e) => {
                            return Err((storage, e));
                        }
                    };
                    let columns = source_column_defs
                        .iter()
                        .map(|ColumnDef { name, .. }| name.to_owned())
                        .collect::<Vec<_>>();

                    let statement = Statement::Insert {
                        table_name: name.to_owned(),
                        columns,
                        source: v.to_owned(),
                    };
                    println!("{:?}", statement);
                    let result = execute(storage, &statement).await;
                    match result {
                        Ok((storage, _)) => {
                            if let Some(schema) = schema {
                                return storage.insert_schema(&schema).await;
                            } else {
                                return Ok((storage, ()));
                            }
                        }
                        Err((storage, Error)) => return Ok((storage, ())),
                    }
                }
            }
            Ok((storage, ()))
        }
        None => {
            let schema = (|| async {
                let schema = Schema {
                    table_name: get_name(name)?.to_string(),
                    column_defs: column_defs.to_vec(),
                    indexes: vec![],
                };

                for column_def in &schema.column_defs {
                    validate(column_def)?;
                }

                match (
                    storage.fetch_schema(&schema.table_name).await?,
                    if_not_exists,
                ) {
                    (None, _) => Ok(Some(schema)),
                    (Some(_), true) => Ok(None),
                    (Some(_), false) => {
                        Err(AlterError::TableAlreadyExists(schema.table_name.to_owned()).into())
                    }
                }
            })()
            .await;

            let schema = match schema {
                Ok(s) => s,
                Err(e) => {
                    return Err((storage, e));
                }
            };

            if let Some(schema) = schema {
                storage.insert_schema(&schema).await
            } else {
                Ok((storage, ()))
            }
        }
    }
}

pub async fn drop_table<T: Debug, U: GStore<T> + GStoreMut<T>>(
    storage: U,
    table_names: &[ObjectName],
    if_exists: bool,
) -> MutResult<U, ()> {
    stream::iter(table_names.iter().map(Ok))
        .try_fold((storage, ()), |(storage, _), table_name| async move {
            let schema = (|| async {
                let table_name = get_name(table_name)?;
                let schema = storage.fetch_schema(table_name).await?;

                if !if_exists {
                    schema.ok_or_else(|| AlterError::TableNotFound(table_name.to_owned()))?;
                }

                Ok(table_name)
            })()
            .await;

            let schema = match schema {
                Ok(s) => s,
                Err(e) => {
                    return Err((storage, e));
                }
            };

            storage.delete_schema(schema).await
        })
        .await
}
