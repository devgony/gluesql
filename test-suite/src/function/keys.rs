use gluesql_core::prelude::Value;

use {
    crate::*,
    gluesql_core::{
        error::{EvaluateError, TranslateError},
        prelude::{Payload, Value::*},
    },
};

test_case!(keys, async move {
    let l = |s: &str| Value::parse_json_list(s).unwrap();
    test! {
       name: "keys() should fetch list of key from map",
       sql: r#"SELECT KEYS('{"a": 1, "b": 2}') as col1"#,
       expected: Ok(
           select_with_null!(
               "col1";
               l(r#"["a", "b"]"#)
           )
       )
    };

    run!(
        r#"
        CREATE TABLE Test (
            map Map
        )"#
    );

    run!("INSERT INTO Test (map) VALUES ('{\"a\": 1, \"b\": 2}')");

    test! {
       name: "keys() should fetch list of key from map column",
       sql: r#"SELECT KEYS(map) as col1 FROM Test"#,
       expected: Ok(
           select_with_null!(
               "col1";
               l(r#"["a", "b"]"#)
           )
       )
    };
});
