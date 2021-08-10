use {
    futures::executor::block_on,
    gluesql::{execute, parse, translate, SledStorage},
};
fn main() {
    let storage = SledStorage::new("data.db").unwrap();
    // let mut glue = Glue::new(storage);
    let sqls = "
        DROP TABLE IF EXISTS DUAL;
        CREATE TABLE DUAL (DUMMY INT);
        INSERT INTO DUAL VALUES (0);
        SELECT Upper('up') from DUAL;
    ";
    // let result = glue.execute(&sqls);
    // println!("Result: {:?}", result);
    // // Result: Ok(Select { labels: ["id"], rows: [Row([I64(200)])] })

    parse(sqls)
        .unwrap()
        .iter()
        .fold(storage, |storage, parsed| {
            let statement = translate(parsed).unwrap();
            let (storage, payload) = block_on(execute(storage, &statement)).unwrap();
            println!("Result: {:?}", payload);
            storage
        });
}
