use {
    super::{context::Context, schema::SchemaKey},
    crate::{
        ast::{ColumnDef, ColumnOption, ColumnOptionDef, Expr, Query, TableAlias, TableFactor},
        data::Schema,
    },
    std::{borrow::Cow, rc::Rc},
};

pub trait Planner<'a> {
    fn get_schema(&'a self, schema_key: &SchemaKey<'a>) -> Option<&'a Schema>;

    fn query(&self, outer_context: Option<Rc<Context<'a>>>, query: Query) -> Query;

    fn subquery_expr(&self, outer_context: Option<Rc<Context<'a>>>, expr: Expr) -> Expr {
        match expr {
            Expr::Identifier(_)
            | Expr::CompoundIdentifier { .. }
            | Expr::Literal(_)
            | Expr::TypedString { .. } => expr,
            Expr::IsNull(expr) => Expr::IsNull(Box::new(self.subquery_expr(outer_context, *expr))),
            Expr::IsNotNull(expr) => {
                Expr::IsNotNull(Box::new(self.subquery_expr(outer_context, *expr)))
            }
            Expr::InList {
                expr,
                list,
                negated,
            } => {
                let list = list
                    .into_iter()
                    .map(|expr| self.subquery_expr(outer_context.as_ref().map(Rc::clone), expr))
                    .collect();
                let expr = Box::new(self.subquery_expr(outer_context, *expr));

                Expr::InList {
                    expr,
                    list,
                    negated,
                }
            }
            Expr::Subquery(query) => Expr::Subquery(Box::new(self.query(outer_context, *query))),
            Expr::Exists { subquery, negated } => Expr::Exists {
                subquery: Box::new(self.query(outer_context, *subquery)),
                negated,
            },
            Expr::InSubquery {
                expr,
                subquery,
                negated,
            } => {
                let expr =
                    Box::new(self.subquery_expr(outer_context.as_ref().map(Rc::clone), *expr));
                let subquery = Box::new(self.query(outer_context, *subquery));

                Expr::InSubquery {
                    expr,
                    subquery,
                    negated,
                }
            }
            Expr::Between {
                expr,
                negated,
                low,
                high,
            } => {
                let expr =
                    Box::new(self.subquery_expr(outer_context.as_ref().map(Rc::clone), *expr));
                let low = Box::new(self.subquery_expr(outer_context.as_ref().map(Rc::clone), *low));
                let high = Box::new(self.subquery_expr(outer_context, *high));

                Expr::Between {
                    expr,
                    negated,
                    low,
                    high,
                }
            }
            Expr::Like {
                expr,
                negated,
                pattern,
            } => {
                let expr =
                    Box::new(self.subquery_expr(outer_context.as_ref().map(Rc::clone), *expr));
                let pattern =
                    Box::new(self.subquery_expr(outer_context.as_ref().map(Rc::clone), *pattern));

                Expr::Like {
                    expr,
                    negated,
                    pattern,
                }
            }
            Expr::ILike {
                expr,
                negated,
                pattern,
            } => {
                let expr =
                    Box::new(self.subquery_expr(outer_context.as_ref().map(Rc::clone), *expr));
                let pattern =
                    Box::new(self.subquery_expr(outer_context.as_ref().map(Rc::clone), *pattern));

                Expr::ILike {
                    expr,
                    negated,
                    pattern,
                }
            }
            Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
                left: Box::new(self.subquery_expr(outer_context.as_ref().map(Rc::clone), *left)),
                op,
                right: Box::new(self.subquery_expr(outer_context, *right)),
            },
            Expr::UnaryOp { op, expr } => Expr::UnaryOp {
                op,
                expr: Box::new(self.subquery_expr(outer_context, *expr)),
            },
            Expr::Cast { expr, data_type } => Expr::Cast {
                expr: Box::new(self.subquery_expr(outer_context, *expr)),
                data_type,
            },
            Expr::Extract { field, expr } => Expr::Extract {
                field,
                expr: Box::new(self.subquery_expr(outer_context, *expr)),
            },
            Expr::Nested(expr) => Expr::Nested(Box::new(self.subquery_expr(outer_context, *expr))),
            Expr::Case {
                operand,
                when_then,
                else_result,
            } => {
                let operand = operand.map(|expr| {
                    Box::new(self.subquery_expr(outer_context.as_ref().map(Rc::clone), *expr))
                });
                let when_then = when_then
                    .into_iter()
                    .map(|(when, then)| {
                        let when = self.subquery_expr(outer_context.as_ref().map(Rc::clone), when);
                        let then = self.subquery_expr(outer_context.as_ref().map(Rc::clone), then);

                        (when, then)
                    })
                    .collect();
                let else_result =
                    else_result.map(|expr| Box::new(self.subquery_expr(outer_context, *expr)));

                Expr::Case {
                    operand,
                    when_then,
                    else_result,
                }
            }
            Expr::Function(_) | Expr::Aggregate(_) => expr,
        }
    }

    fn update_context(
        &'a self,
        next: Option<Rc<Context<'a>>>,
        table_factor: &'a TableFactor,
    ) -> Option<Rc<Context<'a>>> {
        let (name, alias) = match table_factor {
            TableFactor::Table { name, alias, .. } => {
                let alias = alias.as_ref().map(|TableAlias { name, .. }| name.clone());

                (name, alias)
            }
            TableFactor::Derived { .. }
            | TableFactor::Series { .. }
            | TableFactor::Dictionary { .. } => return next,
        };

        // let alias: &'a Option<String> = &alias;

        let schema_key: SchemaKey<'a> = SchemaKey {
            name: Cow::Owned(name.to_owned()),
            // alias: Cow::Owned(alias.clone()),
            alias: alias.as_ref().map(Cow::from),
        };

        let column_defs = match self.get_schema(&schema_key) {
            Some(Schema { column_defs, .. }) => column_defs,
            None => return next,
        };

        let columns = column_defs
            .iter()
            .map(|ColumnDef { name, .. }| name.as_str())
            .collect::<Vec<_>>();

        let primary_key = column_defs
            .iter()
            .find_map(|ColumnDef { name, options, .. }| {
                options
                    .iter()
                    .any(|ColumnOptionDef { option, .. }| {
                        matches!(option, ColumnOption::Unique { is_primary: true })
                    })
                    .then(|| name.as_str())
            });

        let context = Context::new(
            alias.unwrap_or_else(|| name.to_owned()),
            columns,
            primary_key,
            next,
            None,
        );
        Some(Rc::new(context))
    }
}
