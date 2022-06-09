/*
- [x] Add initial test cases at `test-suite/src/inline_view.rs`
- [x] Add `TableFactor::Derived {subquery, alias}` at `core/src/ast/query.rs`
- [x] Fix UnsupportedQueryTableFactor -> Return TableFactor::Derived at `core/src/translate/query.rs`
- [x] Should we separate TableFactor to TableFactorEvaluate and TableFactorTranslate?
    - adhoc Unreachable
- [ ] Impl if relation == Derived, select(subquery) in `select_with_label` at `core/src/executor/select/mod.rs`
- [ ] Sth to do in plan?
*/

use {
    crate::*,
    gluesql_core::prelude::{Payload, Value::*},
};
test_case!(inline_view, async move {
    let test_cases = vec![
        (
            "CREATE TABLE InnerTable (
                id INTEGER,
                name TEXT 
            )",
            Payload::Create,
        ),
        (
            "CREATE TABLE OuterTable (
                id INTEGER,
                name TEXT 
            )",
            Payload::Create,
        ),
        (
            "INSERT INTO InnerTable VALUES (1, 'GLUE'), (2, 'SQL'), (3, 'SQL')",
            Payload::Insert(3),
        ),
        (
            "INSERT INTO OuterTable VALUES (1, 'WORKS!')",
            Payload::Insert(1),
        ),
        (
            "SELECT * FROM InnerTable",
            select!(
                    id  | name
                    I64 | Str;
                    1     "GLUE".to_owned();
                    2     "SQL".to_owned();
                    3     "SQL".to_owned()
            ),
        ),
        (
            "SELECT * FROM (SELECT COUNT(*) AS cnt FROM InnerTable) AS InlineView",
            select!(cnt;I64;3),
        ),
        ( // join
            "SELECT * FROM OuterTable JOIN (SELECT id FROM InnerTable) AS InlineView ON OuterTable.id = InlineView.id",
            select!(cnt;I64;3),
        ),

        // group by
       (
           "SELECT * FROM (
               SELECT name, count(*) as cnt
               FROM InnerTable
               GROUP BY name
            ) AS InlineView",
            select!(
                name             | cnt 
                Str              | I64;
                "GLUE".to_owned()  1;
                "SQL".to_owned()   2
            ),
       ),
        // limit
        (
            "SELECT * FROM (
                SELECT *
                FROM InnerTable
                LIMIT 1
             ) AS InlineView",
             select!(
                 id  | name 
                 I64 | Str;
                 1    "SQL".to_owned()
             ),
        ),
        // offset
        (
            "SELECT * FROM (
                SELECT *
                FROM InnerTable
                OFFSET 1
             ) AS InlineView",
             select!(
                 id  | name 
                 I64 | Str;
                 2    "GLUE".to_owned()
             ),
        ),
        // sort

        // (
        //     // unsupported lateral
        //     "SELECT * FROM OuterTable, (SELECT id FROM InnerTable WHERE InnerTable.id = OuterTable.id) AS InlineView",
        //     select!(cnt;I64;2),
        // ),
    ];
    for (sql, expected) in test_cases {
        test!(Ok(expected), sql);
    }
});
