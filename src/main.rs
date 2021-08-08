use gluesql::{Glue, SledStorage};

fn main() {
    // let storage = SledStorage::new("data/doc-db").unwrap();
    let storage = SledStorage::new("data.db").unwrap();
    let mut glue = Glue::new(storage);
    let sqls = "
        DROP TABLE IF EXISTS Glue;
        CREATE TABLE Glue (id INTEGER);
        INSERT INTO Glue VALUES (100);
        INSERT INTO Glue VALUES (200);
        SELECT * FROM Glue WHERE id > 100;
    ";
    let result = glue.execute(&sqls);
    println!("Result: {:?}", result);
    // Results: [Ok(DropTable), Ok(Create), Ok(Insert(1)), Ok(Insert(1)), Ok(Select { labels: ["id"], rows: [Row([I64(200)])] })]
    // Error: None
}
