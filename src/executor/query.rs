use gluesql::ast::query::Query;

pub enum Queried {
    Select {
        rows: Vec<Row>
    },
    Values {
        values: 
    }
}

pub fn query(source: Query) -> Vec<Row> {
    match source.body {
        SetExpr::Values(Values(values_list)) => values_list
            .iter()
            .map(|values| Row::new(&column_defs, columns, values))
            .collect::<Result<Vec<Row>>>()?,
        SetExpr::Select(select_query) => {
            select(&storage, source, None)
                .await?
                .try_collect::<Vec<_>>()
                .await
        }
    }
}
