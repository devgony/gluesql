use {
    super::{
        select::{NodeData, Prebuild, ValuesData},
        ExprList, ExprNode, LimitNode, OffsetNode, OrderByExprList, OrderByNode, QueryNode,
        TableFactorNode,
    },
    crate::{ast::Expr, result::Result},
};

#[derive(Clone, Debug)]
pub struct ValuesNode<'a> {
    pub values: Vec<ExprList<'a>>,
}

impl<'a> ValuesNode<'a> {
    pub fn order_by<T: Into<OrderByExprList<'a>>>(self, order_by_exprs: T) -> OrderByNode<'a> {
        OrderByNode::new(self, order_by_exprs)
    }

    pub fn offset<T: Into<ExprNode<'a>>>(self, expr: T) -> OffsetNode<'a> {
        OffsetNode::new(self, expr)
    }

    pub fn limit<T: Into<ExprNode<'a>>>(self, expr: T) -> LimitNode<'a> {
        LimitNode::new(self, expr)
    }

    pub fn alias_as(self, table_alias: &'a str) -> TableFactorNode {
        QueryNode::ValuesNode(self).alias_as(table_alias)
    }
}

impl<'a> Prebuild for ValuesNode<'a> {
    fn prebuild(self) -> Result<NodeData> {
        let values = self
            .values
            .into_iter()
            .map(|a| a.try_into())
            .collect::<Result<Vec<Vec<Expr>>>>()?;

        Ok(NodeData::Values(ValuesData {
            values,
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }))
    }
}

pub fn values<'a, T: Into<ExprList<'a>>>(values: Vec<T>) -> ValuesNode<'a> {
    let values: Vec<ExprList> = values.into_iter().map(Into::into).collect();

    ValuesNode { values }
}

#[cfg(test)]
mod tests {
    use {
        super::values,
        crate::ast_builder::{num, test, Build},
    };

    #[test]
    fn values_test() {
        let actual = values(vec![vec![num(7)]]).build();
        let expected = "VALUES(7)";
        test(actual, expected);

        let actual = values(vec!["1, 'a'", "2, 'b'"]).build();
        let expected = "VALUES(1, 'a'), (2, 'b')";
        test(actual, expected);

        let actual = values(vec!["1, 'a'", "2, 'b'"])
            .order_by(vec!["column1 desc"])
            .build();
        let expected = "VALUES(1, 'a'), (2, 'b') ORDER BY column1 desc";
        test(actual, expected);

        let actual = values(vec!["1, 'a'", "2, 'b'"]).offset(1).build();
        let expected = "VALUES(1, 'a'), (2, 'b') offset 1";
        test(actual, expected);

        let actual = values(vec!["1, 'a'", "2, 'b'"]).limit(1).build();
        let expected = "VALUES(1, 'a'), (2, 'b') limit 1";
        test(actual, expected);

        let actual = values(vec!["1, 'a'", "2, 'b'"])
            .alias_as("Sub")
            .select()
            .build();
        let expected = "SELECT * FROM (VALUES(1, 'a'), (2, 'b')) AS Sub";
        test(actual, expected);
    }
}