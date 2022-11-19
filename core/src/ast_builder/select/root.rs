use {
    super::{join::JoinOperatorType, NodeData, Prebuild},
    crate::{
        ast::{SelectItem, TableAlias, TableFactor},
        ast_builder::{
            table::TableType, ExprList, ExprNode, FilterNode, GroupByNode, JoinNode, LimitNode,
            OffsetNode, OrderByExprList, OrderByNode, ProjectNode, SelectItemList, TableNode,
        },
        result::Result,
    },
};

#[derive(Clone)]
pub struct SelectNode<'a> {
    table_node: TableNode<'a>,
    table_alias: Option<String>,
}

impl<'a> SelectNode<'a> {
    pub fn new(table_node: TableNode<'a>, table_alias: Option<String>) -> Self {
        Self {
            table_node,
            table_alias,
        }
    }

    pub fn filter<T: Into<ExprNode<'a>>>(self, expr: T) -> FilterNode<'a> {
        FilterNode::new(self, expr)
    }

    pub fn group_by<T: Into<ExprList<'a>>>(self, expr_list: T) -> GroupByNode<'a> {
        GroupByNode::new(self, expr_list)
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

    pub fn order_by<T: Into<OrderByExprList<'a>>>(self, order_by_exprs: T) -> OrderByNode<'a> {
        OrderByNode::new(self, order_by_exprs)
    }

    pub fn join(self, table_name: &str) -> JoinNode<'a> {
        JoinNode::new(self, table_name.to_owned(), None, JoinOperatorType::Inner)
    }

    pub fn join_as(self, table_name: &str, alias: &str) -> JoinNode<'a> {
        JoinNode::new(
            self,
            table_name.to_owned(),
            Some(alias.to_owned()),
            JoinOperatorType::Inner,
        )
    }

    pub fn left_join(self, table_name: &str) -> JoinNode<'a> {
        JoinNode::new(self, table_name.to_owned(), None, JoinOperatorType::Left)
    }

    pub fn left_join_as(self, table_name: &str, alias: &str) -> JoinNode<'a> {
        JoinNode::new(
            self,
            table_name.to_owned(),
            Some(alias.to_owned()),
            JoinOperatorType::Left,
        )
    }
}

impl<'a> Prebuild for SelectNode<'a> {
    fn prebuild(self) -> Result<NodeData> {
        let alias = self.table_alias.map(|name| TableAlias {
            name,
            columns: Vec::new(),
        });

        let alias_or_name = match alias.clone() {
            Some(alias) => alias,
            None => TableAlias {
                name: self.table_node.table_name.clone(),
                columns: Vec::new(),
            },
        };

        let relation = match self.table_node.table_type {
            TableType::Table => TableFactor::Table {
                name: self.table_node.table_name,
                alias,
                index: None,
            },
            TableType::Dictionary(dict) => TableFactor::Dictionary {
                dict,
                alias: alias_or_name,
            },
            TableType::Series if self.table_node.args.is_some() => TableFactor::Series {
                alias: alias_or_name,
                size: self.table_node.args.unwrap().try_into()?,
            },
            TableType::Derived => todo!(),
            _ => todo!(),
        };

        Ok(NodeData {
            projection: vec![SelectItem::Wildcard],
            relation,
            filter: None,
            group_by: vec![],
            having: None,
            order_by: vec![],
            offset: None,
            limit: None,
            joins: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ast_builder::{table, test, Build};

    #[test]
    fn select() {
        // select node -> build
        let actual = table("App").select().build();
        let expected = "SELECT * FROM App";
        test(actual, expected);

        let actual = table("Item").alias_as("i").select().build();
        let expected = "SELECT * FROM Item i";
        test(actual, expected);
    }
}
