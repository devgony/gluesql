use {
    gluesql_core::{
        executor::FetchError,
        prelude::{Glue, Payload, Value},
        result::Error,
    },
    gluesql_jsonl_storage::JsonlStorage,
    serde_json::json,
    std::{
        fs::{remove_dir_all, File},
        io::Write,
    },
    test_suite::{select_map, test},
};

#[test]
fn json_dml() {
    let path = "tmp/json_dml";
    if let Err(e) = remove_dir_all(path) {
        println!("fs::remove_file {:?}", e);
    };
    let jsonl_storage = JsonlStorage::new(path).unwrap();
    let mut glue = Glue::new(jsonl_storage);

    let dir = format!("{path}/JsonDML.json");
    let mut file = File::create(dir).unwrap();
    let data = r#"[
  {
    "id": 1,
    "notice": "should keep this array of jsons format"
  }
]
"#;
    write!(file, "{data}").unwrap();

    let cases = vec![
        (
            glue.execute(r#"INSERT INTO JsonDML VALUES ('{"id": 2, "notice": "appended json"}')"#),
            Ok(Payload::Insert(1)),
        ),
        (
            glue.execute("SELECT * FROM JsonDML"),
            Ok(select_map![
                json!({
                  "id": 1,
                  "notice": "should keep this array of jsons format"
                }),
                json!({
                  "id": 2,
                  "notice": "appended json"
                })
            ]),
        ),
        (
            glue.execute("UPDATE JsonDML SET notice = 'updated' WHERE id = 2"),
            Ok(Payload::Update(1)),
        ),
        (
            glue.execute("SELECT * FROM JsonDML WHERE id = 2"),
            Ok(select_map![json!({
              "id": 2,
              "notice": "updated"
            })]),
        ),
        (
            glue.execute("DELETE FROM JsonDML WHERE id = 2"),
            Ok(Payload::Delete(1)),
        ),
        (
            glue.execute("SELECT * FROM JsonDML"),
            Ok(select_map![json!({
              "id": 1,
              "notice": "should keep this array of jsons format"
            })]),
        ),
        // (
        //     glue.execute("SELECT COUNT(*) FROM GLUE_TABLES WHERE TABLE_NAME = 'JsonDML'"),
        //     Ok(Payload::Select {
        //         labels: vec!["COUNT(*)".to_owned()],
        //         rows: vec![vec![Value::I64(1)]],
        //     }),
        // ),
        (glue.execute("DROP TABLE JsonDML"), Ok(Payload::DropTable)),
        (
            glue.execute("SELECT COUNT(*) FROM GLUE_TABLES WHERE TABLE_NAME = 'JsonDML'"),
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
