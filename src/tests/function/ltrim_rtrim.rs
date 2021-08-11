use crate::*;

test_case!(ltrim_rtrim, async move {
    use Value::Str;
    let test_cases = vec![
        ("CREATE TABLE Item (name TEXT)", Ok(Payload::Create)),
        (
            r#"INSERT INTO Item VALUES (" Blop mc"), (" B"), (" Steven")"#,
            Ok(Payload::Insert(3)),
        ),
        (
            r#"SELECT Ltrim(name, ' B') AS test FROM Item"#,
            Ok(select!(
                "test"
                Str;
                "Blop mc".to_owned();
                "B".to_owned();
                "Steven ".to_owned()
            )),
        ),
    ];
    for (sql, expected) in test_cases {
        test!(expected, sql);
    }
});
