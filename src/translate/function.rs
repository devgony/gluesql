use {
    super::{expr::translate_expr, translate_object_name, TranslateError},
    crate::{
        ast::{Aggregate, Expr, Function, ObjectName},
        result::Result,
    },
    sqlparser::ast::{Function as SqlFunction, FunctionArg as SqlFunctionArg},
};

pub fn translate_function(sql_function: &SqlFunction) -> Result<Expr> {
    let SqlFunction { name, args, .. } = sql_function;
    let name = {
        let ObjectName(names) = translate_object_name(name);

        names[0].to_uppercase()
    };
    let args = args
        .iter()
        .map(|arg| match arg {
            SqlFunctionArg::Named { .. } => {
                Err(TranslateError::NamedFunctionArgNotSupported.into())
            }
            SqlFunctionArg::Unnamed(expr) => Ok(expr),
        })
        .collect::<Result<Vec<_>>>()?;

    let check_len = |name, found, expected| -> Result<_> {
        if found == expected {
            Ok(())
        } else {
            Err(TranslateError::FunctionArgsLengthNotMatching {
                name,
                expected,
                found,
            }
            .into())
        }
    };

    macro_rules! aggr {
        ($aggregate: expr) => {{
            check_len(name, args.len(), 1)?;

            translate_expr(args[0])
                .map($aggregate)
                .map(Box::new)
                .map(Expr::Aggregate)
        }};
    }

    macro_rules! func_with_one_arg {
        ($func: expr) => {{
            check_len(name, args.len(), 1)?;

            translate_expr(args[0])
                .map($func)
                .map(Box::new)
                .map(Expr::Function)
        }};
    }
    macro_rules! func_with_two_arg {
        ($func: expr) => {{
            let result = match args.len() {
                1 => Ok((translate_expr(args[0])?, None)),
                2 => Ok((translate_expr(args[0])?, translate_expr(args[1]).map(Some)?)),
                n => Err(TranslateError::FunctionArgsLengthRangeNotMatching {
                    name: "LTRIM".to_owned(),
                    min: 1,
                    max: 2,
                    found: n,
                }),
            };
            let (expr, chars) = result?;
            // {expr, chars}.map($func).map(Box::new).map(Expr::Function)
            Ok(Expr::Function(Box::new(Function::Rtrim { expr, chars })))
        }};
    }
    // let check_len2 = |args: Vec<&Expr, Global>| match args.len() {
    //     1 => Ok((translate_expr(args[0])?, None)),
    //     2 => Ok((translate_expr(args[0])?, translate_expr(args[1]).map(Some)?)),
    //     n => Err(TranslateError::FunctionArgsLengthRangeNotMatching {
    //         name: "LTRIM".to_owned(),
    //         min: 1,
    //         max: 2,
    //         found: n,
    //     }),
    // };

    match name.as_str() {
        "LOWER" => func_with_one_arg!(Function::Lower),
        "UPPER" => func_with_one_arg!(Function::Upper),
        "LEFT" => {
            check_len(name, args.len(), 2)?;

            let expr = translate_expr(args[0])?;
            let size = translate_expr(args[1])?;

            Ok(Expr::Function(Box::new(Function::Left { expr, size })))
        }
        "RIGHT" => {
            check_len(name, args.len(), 2)?;

            let expr = translate_expr(args[0])?;
            let size = translate_expr(args[1])?;

            Ok(Expr::Function(Box::new(Function::Right { expr, size })))
        }
        "CEIL" => func_with_one_arg!(Function::Ceil),
        "ROUND" => func_with_one_arg!(Function::Round),
        "FLOOR" => func_with_one_arg!(Function::Floor),
        "GCD" => {
            check_len(name, args.len(), 2)?;

            let left = translate_expr(args[0])?;
            let right = translate_expr(args[1])?;

            Ok(Expr::Function(Box::new(Function::Gcd { left, right })))
        }
        "LCM" => {
            check_len(name, args.len(), 2)?;

            let left = translate_expr(args[0])?;
            let right = translate_expr(args[1])?;

            Ok(Expr::Function(Box::new(Function::Lcm { left, right })))
        }
        "LTRIM" => {
            let result = match args.len() {
                1 => Ok((translate_expr(args[0])?, None)),
                2 => Ok((translate_expr(args[0])?, translate_expr(args[1]).map(Some)?)),
                n => Err(TranslateError::FunctionArgsLengthRangeNotMatching {
                    name: "LTRIM".to_owned(),
                    min: 1,
                    max: 2,
                    found: n,
                }),
            };
            let (expr, chars) = result?;
            Ok(Expr::Function(Box::new(Function::Ltrim { expr, chars })))
        }
        "RTRIM" => func_with_two_arg!(Function::Rtrim),
        "COUNT" => aggr!(Aggregate::Count),
        "SUM" => aggr!(Aggregate::Sum),
        "MIN" => aggr!(Aggregate::Min),
        "MAX" => aggr!(Aggregate::Max),
        "TRIM" => func_with_one_arg!(Function::Trim),
        "DIV" => {
            check_len(name, args.len(), 2)?;

            let dividend = translate_expr(args[0])?;
            let divisor = translate_expr(args[1])?;

            Ok(Expr::Function(Box::new(Function::Div {
                dividend,
                divisor,
            })))
        }
        "MOD" => {
            check_len(name, args.len(), 2)?;

            let dividend = translate_expr(args[0])?;
            let divisor = translate_expr(args[1])?;

            Ok(Expr::Function(Box::new(Function::Mod {
                dividend,
                divisor,
            })))
        }
        _ => Err(TranslateError::UnsupportedFunction(name).into()),
    }
}
