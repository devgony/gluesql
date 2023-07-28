use {
    crate::*,
    gluesql_core::{ast_builder::*, executor::Payload, prelude::Value::*},
};

test_case!(expr_alias_as, async move {
    let glue = get_glue!();

    // Create table - Number
    let actual = table("Number")
        .create_table()
        .add_column("id INTEGER")
        .add_column("number INTEGER")
        .execute(glue)
        .await;
    let expected = Ok(Payload::Create);
    assert_eq!(actual, expected, "create table - Number");

    // Insert a row into the Number
    let actual = table("Number")
        .insert()
        .values(vec!["0, 0", "1, 3", "2, 4", "3, 29"])
        .execute(glue)
        .await;
    let expected = Ok(Payload::Insert(4));
    assert_eq!(actual, expected, "insert into Number");

    // Example Using ABS
    let actual = values(vec!["0, 0", "1, -3", "2, 4", "3, -29"])
        .alias_as("number")
        .select()
        .project("column1")
        .project(col("column1").alias_as("c1"))
        .project(abs("column2").alias_as("c2_abs"))
        .project(col("column1").add(abs(col("column2"))).alias_as("c3"))
        // .project("(select 1) AS col3") // TODO: test scala subquery later
        .execute(glue)
        .await;
    let expected = Ok(select!(
        column1 | c1  | c2_abs | c3;
        I64     | I64 | I64    | I64;
        0         0     0        0;
        1         1     3        4;
        2         2     4        6;
        3         3     29       32
    ));
    assert_eq!(actual, expected, "Example Using ABS");
});
