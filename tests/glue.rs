#![cfg(any(feature = "gluesql_memory_storage", feature = "gluesql_sled_storage"))]
use {
    gluesql_core::{
        executor::Payload,
        prelude::{Glue, Value},
        store::{GStore, GStoreMut},
    },
    std::fmt::Debug,
};

fn basic<T: Debug, U: GStore<T> + GStoreMut<T>>(mut glue: Glue<T, U>) {
    assert_eq!(
        glue.execute("DROP TABLE IF EXISTS api_test"),
        Ok(Payload::DropTable)
    );

    assert_eq!(
        glue.execute(
            "CREATE TABLE api_test (id INTEGER, name TEXT, nullable TEXT NULL, is BOOLEAN)"
        ),
        Ok(Payload::Create)
    );

    assert_eq!(
        glue.execute(
            "
                INSERT INTO
                    api_test (id, name, nullable, is)
                VALUES
                    (1, 'test1', 'not null', TRUE),
                    (2, 'test2', NULL, FALSE)"
        ),
        Ok(Payload::Insert(2))
    );

    assert_eq!(
        glue.execute("SELECT id, name, is FROM api_test"),
        Ok(Payload::Select {
            labels: vec![String::from("id"), String::from("name"), String::from("is")],
            rows: vec![
                vec![
                    Value::I64(1),
                    Value::Str(String::from("test1")),
                    Value::Bool(true)
                ],
                vec![
                    Value::I64(2),
                    Value::Str(String::from("test2")),
                    Value::Bool(false)
                ]
            ]
        })
    );
}

async fn basic_async<T: Debug, U: GStore<T> + GStoreMut<T>>(mut glue: Glue<T, U>) {
    assert_eq!(
        glue.execute_async("DROP TABLE IF EXISTS api_test").await,
        Ok(Payload::DropTable)
    );

    assert_eq!(
        glue.execute_async(
            "CREATE TABLE api_test (id INTEGER, name TEXT, nullable TEXT NULL, is BOOLEAN)"
        )
        .await,
        Ok(Payload::Create)
    );
}

#[cfg(feature = "gluesql_sled_storage")]
#[test]
fn sled_basic() {
    use gluesql_sled_storage::{sled, SledStorage};

    let config = sled::Config::default()
        .path("data/using_config")
        .temporary(true);

    let storage = SledStorage::try_from(config).unwrap();
    let glue = Glue::new(storage);

    basic(glue);
}

#[cfg(feature = "gluesql_memory_storage")]
#[test]
fn memory_basic() {
    use gluesql_memory_storage::MemoryStorage;

    let storage = MemoryStorage::default();
    let glue = Glue::new(storage);

    basic(glue);
}

#[cfg(feature = "gluesql_memory_storage")]
#[test]
fn memory_basic_async() {
    use futures::executor::block_on;
    use gluesql_memory_storage::MemoryStorage;

    let storage = MemoryStorage::default();
    let glue = Glue::new(storage);

    block_on(basic_async(glue));
}