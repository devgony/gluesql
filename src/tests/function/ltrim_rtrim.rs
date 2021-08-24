use crate::*;

test_case!(ltrim_rtrim, async move {
    use Value::Str;
    let test_cases = vec![
        ("CREATE TABLE Item (name TEXT)", Ok(Payload::Create)),
        (
            r#"INSERT INTO Item VALUES (" zzzytest"), ("testxxzx ")"#,
            Ok(Payload::Insert(2)),
        ),
        (
            r#"SELECT Ltrim(name) AS test FROM Item"#,
            Ok(select!(
                "test"
                Str;
                "zzzytest".to_owned();
                "testxxzx ".to_owned()
            )),
        ),
        (
            r#"SELECT Ltrim(name, ' xyz') AS test FROM Item"#,
            Ok(select!(
                "test"
                Str;
                "test".to_owned();
                "testxxzx ".to_owned()
            )),
        ),
        (
            r#"SELECT Rtrim(name) AS test FROM Item"#,
            Ok(select!(
                "test"
                Str;
                " zzzytest".to_owned();
                "testxxzx".to_owned()
            )),
        ),
        (
            r#"SELECT Rtrim(name, 'xyz ') AS test FROM Item"#,
            Ok(select!(
                "test"
                Str;
                " zzzytest".to_owned();
                "test".to_owned()
            )),
        ),
        (
            r#"SELECT Ltrim(1) AS test FROM Item"#,
            Err(EvaluateError::FunctionRequiresStringValue("LTRIM".to_owned()).into()),
        ),
        (
            r#"SELECT Ltrim(name, 1) AS test FROM Item"#,
            Err(EvaluateError::FunctionRequiresStringValue("LTRIM".to_owned()).into()),
        ),
        (
            r#"SELECT Rtrim(1) AS test FROM Item"#,
            Err(EvaluateError::FunctionRequiresStringValue("RTRIM".to_owned()).into()),
        ),
        (
            r#"SELECT Rtrim(name, 1) AS test FROM Item"#,
            Err(EvaluateError::FunctionRequiresStringValue("RTRIM".to_owned()).into()),
        ),
        (
            "CREATE TABLE NullTest (name TEXT null)",
            Ok(Payload::Create),
        ),
        (
            r#"INSERT INTO NullTest VALUES (null)"#,
            Ok(Payload::Insert(1)),
        ),
        (
            r#"SELECT Ltrim(name) AS test FROM NullTest"#,
            Ok(select_with_null!(test; Value::Null)),
        ),
        (
            r#"SELECT Rtrim(name) AS test FROM NullTest"#,
            Ok(select_with_null!(test; Value::Null)),
        ),
    ];
    for (sql, expected) in test_cases {
        test!(expected, sql);
    }
});
