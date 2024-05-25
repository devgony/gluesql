use {
    crate::*,
    gluesql_core::{
        error::{ExecuteError, InsertError, UpdateError},
        executor::{AlterError, Referencing},
        prelude::Payload,
    },
};

test_case!(foreign_key, {
    let g = get_tester!();

    g.run(
        "
        CREATE TABLE ReferencedTableWithoutPK (
            id INTEGER,
            name TEXT,
        );
    ",
    )
    .await;

    g.named_test(
        "Create table with foreign key should be failed if referenced table does not have primary key",
        "
        CREATE TABLE ReferencingTable (
            id INT, name TEXT,
            referenced_table_id INT,
            FOREIGN KEY(referenced_table_id) REFERENCES ReferencedTableWithoutPK(id)
        );
        ",
        Err(AlterError::ReferencingNonPKColumn {
            referenced_table: "ReferencedTableWithoutPK".to_owned(),
            referenced_column: "id".to_owned(),
        }
        .into()),
    )
    .await;

    g.run(
        "
        CREATE TABLE ReferencedTableWithUnique (
            id INTEGER UNIQUE,
            name TEXT,
        );
    ",
    )
    .await;

    g.named_test(
        "Create table with foreign key should be failed if referenced table has only Unique constraint",
        "
        CREATE TABLE ReferencingTableUnique (
            id INT,
            name TEXT,
            referenced_table_id INT,
            FOREIGN KEY(referenced_table_id) REFERENCES ReferencedTableWithUnique(id)
        );
        ",
        Err(AlterError::ReferencingNonPKColumn {
            referenced_table: "ReferencedTableWithUnique".to_owned(),
            referenced_column: "id".to_owned(),
        }
        .into()),
    )
    .await;

    g.run(
        "
        CREATE TABLE ReferencedTableWithPK (
            id INTEGER PRIMARY KEY,
            name TEXT,
        );
    ",
    )
    .await;

    g.run(
        "
        CREATE TABLE ReferencingTable (
            id INT,
            name TEXT,
            referenced_table_id INT,
            FOREIGN KEY(referenced_table_id) REFERENCES ReferencedTableWithPK (id)
        );
    ",
    )
    .await;

    g.named_test(
        "If there is no referenced table, insert should fail",
        "INSERT INTO ReferencingTable VALUES (1, 'orphan', 1);",
        Err(InsertError::CannotFindReferencedValue {
            table_name: "ReferencedTableWithPK".to_owned(),
            column_name: "id".to_owned(),
            referenced_value: "1".to_owned(),
        }
        .into()),
    )
    .await;

    g.named_test(
        "Even If there is no referenced table, NULL should be inserted",
        "INSERT INTO ReferencingTable VALUES (1, 'Null is independent', NULL);",
        Ok(Payload::Insert(1)),
    )
    .await;

    g.run("INSERT INTO ReferencedTableWithPK VALUES (1, 'referenced_table1');")
        .await;

    g.named_test(
        "With valid referenced table, insert should succeed",
        "INSERT INTO ReferencingTable VALUES (2, 'referencing_table with referenced_table', 1);",
        Ok(Payload::Insert(1)),
    )
    .await;

    g.named_test(
        "If there is no referenced table, update should fail",
        "UPDATE ReferencingTable SET referenced_table_id = 2 WHERE id = 2;",
        Err(UpdateError::CannotFindReferencedValue {
            table_name: "ReferencedTableWithPK".to_owned(),
            column_name: "id".to_owned(),
            referenced_value: "2".to_owned(),
        }
        .into()),
    )
    .await;

    g.named_test(
        "Even If there is no referenced table, it should be able to update to NULL",
        "UPDATE ReferencingTable SET referenced_table_id = NULL WHERE id = 2;",
        Ok(Payload::Update(1)),
    )
    .await;

    g.named_test(
        "With valid referenced table, update should succeed",
        "UPDATE ReferencingTable SET referenced_table_id = 1 WHERE id = 2;",
        Ok(Payload::Update(1)),
    )
    .await;

    g.named_test(
        "Delete referenced table should fail if referencing table exists (by default: NO ACTION and gets error)",
        "DELETE FROM ReferencedTableWithPK WHERE id = 1;",
        Err(ExecuteError::ReferencingColumnExists("ReferencingTable.referenced_table_id".to_owned()).into()),
    )
    .await;

    g.named_test(
        "Deleting referencing table does not care referenced tables",
        "DELETE FROM ReferencingTable WHERE id = 2;",
        Ok(Payload::Delete(1)),
    )
    .await;

    g.named_test(
        "Cannot drop referenced table if referencing table exists",
        "DROP TABLE ReferencedTableWithPK;",
        Err(AlterError::CannotDropTableWitnReferencing {
            referenced_table_name: "ReferencedTableWithPK".to_owned(),
            referencings: vec![Referencing {
                table_name: "ReferencingTable".to_owned(),
                constraint_name: "FK_referenced_table_id-ReferencedTableWithPK_id".to_owned(),
            }],
        }
        .into()),
    )
    .await;

    g.named_test(
        "Drop table with cascade should drop both table and constraint",
        "DROP TABLE ReferencedTableWithPK CASCADE;",
        Ok(Payload::DropTable),
    )
    .await;
});
