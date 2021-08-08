#![cfg(feature = "sled-storage")]
use {
    crate::{
        ast::Statement, execute, parse, plan, storages::SledStorage, translate, Payload, Result,
    },
    futures::executor::block_on,
};

pub struct Glue {
    pub storage: Option<SledStorage>,
}

impl Glue {
    pub fn new(storage: SledStorage) -> Self {
        let storage = Some(storage);

        Self { storage }
    }

    pub fn plan(&self, sql: &str) -> Vec<Result<Statement>> {
        let statements: Vec<Result<Statement>>;
        let parsed = parse(sql).unwrap();
        let storage = self.storage.as_ref().unwrap();
        parsed.iter().try_fold(storage, |storage, parsed| {
            // let translated = translate(&parsed);
            match translate(&parsed) {
                Ok(statement) => {
                    statements.push(block_on(plan(storage, statement)));
                    return Ok(statement);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        });
        return statements;
    }

    pub fn execute_stmt(&mut self, statements: Vec<Statement>) -> Result<Payload> {
        let storage = self.storage.take().unwrap();
        let mut payload: Option<Payload> = None;
        let result = statements.iter().try_fold(storage, |storage, statement| {
            match block_on(execute(storage, &statement)) {
                Ok((s, p)) => {
                    payload = Some(p);
                    return Ok(s);
                }
                Err((_s, e)) => {
                    return Err(e);
                }
            }
        });
        self.storage = Some(result?);
        return Ok(payload.unwrap());
    }

    pub fn execute(&mut self, sql: &str) -> Result<Payload> {
        let statements = self.plan(sql)?;

        self.execute_stmt(statements)
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::{Glue, Payload, Row, SledStorage, Value},
        std::convert::TryFrom,
    };

    #[test]
    fn eq() {
        let config = sled::Config::default()
            .path("data/using_config")
            .temporary(true);

        let sled = SledStorage::try_from(config).unwrap();
        let mut glue = Glue::new(sled);

        assert_eq!(
            glue.execute("DROP TABLE IF EXISTS api_test"),
            Ok(Payload::DropTable)
        );
        assert_eq!(
            glue.execute("CREATE TABLE api_test (id INTEGER PRIMARY KEY, name TEXT, nullable TEXT NULL, is BOOLEAN)"),
            Ok(Payload::Create)
        );
        assert_eq!(
            glue.execute("INSERT INTO api_test (id, name, nullable, is) VALUES (1, 'test1', 'not null', TRUE), (2, 'test2', NULL, FALSE)"),
            Ok(Payload::Insert(2))
        );

        assert_eq!(
            glue.execute("SELECT id, name, is FROM api_test"),
            Ok(Payload::Select {
                labels: vec![String::from("id"), String::from("name"), String::from("is")],
                rows: vec![
                    Row(vec![
                        Value::I64(1),
                        Value::Str(String::from("test1")),
                        Value::Bool(true)
                    ]),
                    Row(vec![
                        Value::I64(2),
                        Value::Str(String::from("test2")),
                        Value::Bool(false)
                    ])
                ]
            })
        );
    }
}
