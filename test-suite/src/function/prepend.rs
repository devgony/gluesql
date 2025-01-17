use {
    crate::*,
    gluesql_core::{
        error::EvaluateError,
        prelude::{Payload, Value::*},
    },
};

test_case!(prepend, async move {
    run!(
        "
        CREATE TABLE Prepend (
            id INTEGER,
            items LIST,
            element INTEGER,
            element2 TEXT
        );
    "
    );
    run!(
        r#"
            INSERT INTO Prepend VALUES
            (1, '[1, 2, 3]',0, 'Foo');
        "#
    );
    test!(
        r#"select prepend(items, element) as myprepend from Prepend;"#,
        Ok(select!(
           myprepend
           List;
           vec![I64(0), I64(1), I64(2), I64(3)]
        ))
    );
    test!(
        r#"select prepend(items, element2) as myprepend from Prepend;"#,
        Ok(select!(
           myprepend
           List;
           vec![Str("Foo".into()), I64(1), I64(2), I64(3)]
        ))
    );

    test!(
        r#"select prepend(element, element2) as myprepend from Prepend"#,
        Err(EvaluateError::ListTypeRequired.into())
    );

    test!(
        r#"CREATE TABLE Foo (
                elements LIST
            );"#,
        Ok(Payload::Create)
    );

    run!(
        r#"
            INSERT INTO Foo VALUES (PREPEND(CAST('[1, 2, 3]' AS LIST), 0));
        "#
    );
    test!(
        r#"select elements as myprepend from Foo;"#,
        Ok(select!(
           myprepend
           List;
           vec![I64(0), I64(1), I64(2), I64(3)]
        ))
    );
});
