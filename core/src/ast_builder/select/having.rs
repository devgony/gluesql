use {
    super::{NodeData, Prebuild},
    crate::{
        ast_builder::{
            table::TableType, ExprNode, GroupByNode, LimitNode, OffsetNode, OrderByExprList,
            OrderByNode, ProjectNode, QueryNode, SelectItemList, TableAliasNode, TableNode,
        },
        result::Result,
    },
};

#[derive(Clone)]
pub enum PrevNode<'a> {
    GroupBy(GroupByNode<'a>),
}

impl<'a> Prebuild for PrevNode<'a> {
    fn prebuild(self) -> Result<NodeData> {
        match self {
            Self::GroupBy(node) => node.prebuild(),
        }
    }
}

impl<'a> From<GroupByNode<'a>> for PrevNode<'a> {
    fn from(node: GroupByNode<'a>) -> Self {
        PrevNode::GroupBy(node)
    }
}

#[derive(Clone)]
pub struct HavingNode<'a> {
    prev_node: PrevNode<'a>,
    expr: ExprNode<'a>,
}

impl<'a> HavingNode<'a> {
    pub fn new<N: Into<PrevNode<'a>>, T: Into<ExprNode<'a>>>(prev_node: N, expr: T) -> Self {
        Self {
            prev_node: prev_node.into(),
            expr: expr.into(),
        }
    }

    pub fn offset<T: Into<ExprNode<'a>>>(self, expr: T) -> OffsetNode<'a> {
        OffsetNode::new(self, expr)
    }

    pub fn limit<T: Into<ExprNode<'a>>>(self, expr: T) -> LimitNode<'a> {
        LimitNode::new(self, expr)
    }

    pub fn project<T: Into<SelectItemList<'a>>>(self, select_items: T) -> ProjectNode<'a> {
        ProjectNode::new(self, select_items)
    }

    pub fn order_by<T: Into<OrderByExprList<'a>>>(self, expr_list: T) -> OrderByNode<'a> {
        OrderByNode::new(self, expr_list)
    }

    pub fn alias_as(self, table_alias: &'a str) -> TableAliasNode {
        let table_node = TableNode {
            table_name: table_alias.to_owned(),
            table_type: TableType::Derived {
                subquery: Box::new(QueryNode::HavingNode(self)),
                alias: table_alias.to_owned(),
            },
        };

        TableAliasNode {
            table_node,
            table_alias: table_alias.to_owned(),
        }
    }
}

impl<'a> Prebuild for HavingNode<'a> {
    fn prebuild(self) -> Result<NodeData> {
        let mut select_data = self.prev_node.prebuild()?;
        select_data.having = Some(self.expr.try_into()?);

        Ok(select_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::ast_builder::{table, test, Build};

    #[test]
    fn having() {
        // group by node -> having node -> offset node
        let actual = table("Bar")
            .select()
            .filter("id IS NULL")
            .group_by("id, (a + name)")
            .having("COUNT(id) > 10")
            .offset(10)
            .build();
        let expected = "
            SELECT * FROM Bar
            WHERE id IS NULL
            GROUP BY id, (a + name)
            HAVING COUNT(id) > 10
            OFFSET 10
        ";
        test(actual, expected);

        // group by node -> having node -> limit node
        let actual = table("Bar")
            .select()
            .filter("id IS NULL")
            .group_by("id, (a + name)")
            .having("COUNT(id) > 10")
            .limit(10)
            .build();
        let expected = "
            SELECT * FROM Bar
            WHERE id IS NULL
            GROUP BY id, (a + name)
            HAVING COUNT(id) > 10
            LIMIT 10
            ";
        test(actual, expected);

        // group by node -> having node -> project node
        let actual = table("Bar")
            .select()
            .filter("id IS NULL")
            .group_by("id, (a + name)")
            .having("COUNT(id) > 10")
            .project(vec!["id", "(a + name) AS b", "COUNT(id) AS c"])
            .build();
        let expected = "
            SELECT id, (a + name) AS b, COUNT(id) AS c
            FROM Bar
            WHERE id IS NULL
            GROUP BY id, (a + name)
            HAVING COUNT(id) > 10
        ";
        test(actual, expected);

        // group by node -> having node -> build
        let actual = table("Bar")
            .select()
            .filter("id IS NULL")
            .group_by("id, (a + name)")
            .having("COUNT(id) > 10")
            .build();
        let expected = "
                SELECT * FROM Bar
                WHERE id IS NULL
                GROUP BY id, (a + name)
                HAVING COUNT(id) > 10
            ";
        test(actual, expected);
    }
}
