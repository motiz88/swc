use crate::{helpers::Helpers, util::ExprFactory};
use ast::*;
use std::{iter, sync::Arc};
use swc_common::{Fold, FoldWith, Spanned, DUMMY_SP};
#[cfg(test)]
mod tests;

#[derive(Default, Clone)]
pub struct TemplateLiteral {
    pub helpers: Arc<Helpers>,
}
pass_from!(TemplateLiteral, |helpers| TemplateLiteral { helpers });

impl Fold<Expr> for TemplateLiteral {
    fn fold(&mut self, e: Expr) -> Expr {
        let e = e.fold_children(self);

        match e {
            Expr::Tpl(Tpl { exprs, quasis, .. }) => {
                assert!(quasis.len() == exprs.len() + 1);

                // TODO: Optimize

                // This makes result of addition string
                let mut obj: Box<Expr> = box Lit::Str(
                    quasis[0]
                        .cooked
                        .clone()
                        .unwrap_or_else(|| quasis[0].raw.clone()),
                )
                .into();

                for i in 0..quasis.len() + exprs.len() {
                    if i == 0 {
                        continue;
                    }

                    let idx = i / 2;

                    let expr = if i % 2 == 0 {
                        // Quasis
                        if quasis[idx].raw.is_empty() {
                            // Skip empty ones
                            continue;
                        }
                        box Lit::Str(
                            quasis[idx]
                                .cooked
                                .clone()
                                .unwrap_or_else(|| quasis[idx].raw.clone()),
                        )
                        .into()
                    } else {
                        // Expression
                        exprs[idx].clone()
                    };

                    obj = box Expr::Bin(BinExpr {
                        span: expr.span(),
                        left: obj,
                        op: op!(bin, "+"),
                        right: expr.into(),
                    });
                }
                return *obj;
            }

            Expr::TaggedTpl(TaggedTpl {
                tag, exprs, quasis, ..
            }) => {
                assert!(quasis.len() == exprs.len() + 1);
                self.helpers.tagged_template_literal();

                let fn_ident = private_ident!("_templateObject");

                let tpl_obj_fn = {
                    Expr::Fn(FnExpr {
                        ident: Some(fn_ident.clone()),
                        function: Function {
                            span: DUMMY_SP,
                            is_async: false,
                            is_generator: false,
                            params: vec![],
                            body: {
                                // const data = _taggedTemplateLiteral(["first", "second"]);
                                let data_decl = VarDecl {
                                    span: DUMMY_SP,
                                    kind: VarDeclKind::Const,
                                    declare: false,
                                    decls: vec![VarDeclarator {
                                        span: DUMMY_SP,
                                        name: quote_ident!("data").into(),
                                        definite: false,
                                        init: Some(box Expr::Call(CallExpr {
                                            span: DUMMY_SP,
                                            callee: quote_ident!("_taggedTemplateLiteral")
                                                .as_callee(),
                                            args: {
                                                let has_escape = quasis.iter().any(|s| {
                                                    s.cooked
                                                        .as_ref()
                                                        .map(|s| s.has_escape)
                                                        .unwrap_or(true)
                                                });

                                                let raw = if has_escape {
                                                    Some(
                                                        ArrayLit {
                                                            span: DUMMY_SP,
                                                            elems: quasis
                                                                .iter()
                                                                .cloned()
                                                                .map(|elem| {
                                                                    Lit::Str(elem.raw).as_arg()
                                                                })
                                                                .map(Some)
                                                                .collect(),
                                                        }
                                                        .as_arg(),
                                                    )
                                                } else {
                                                    None
                                                };

                                                iter::once(
                                                    ArrayLit {
                                                        span: DUMMY_SP,
                                                        elems: quasis
                                                            .into_iter()
                                                            .map(|elem| {
                                                                Lit::Str(
                                                                    elem.cooked.unwrap_or(elem.raw),
                                                                )
                                                                .as_arg()
                                                            })
                                                            .map(Some)
                                                            .collect(),
                                                    }
                                                    .as_arg(),
                                                )
                                                .chain(raw)
                                                .collect()
                                            },
                                            type_args: Default::default(),
                                        })),
                                    }],
                                };

                                // _templateObject2 = function () {
                                //     return data;
                                // };
                                let assign_expr = {
                                    Expr::Assign(AssignExpr {
                                        span: DUMMY_SP,
                                        left: PatOrExpr::Pat(box fn_ident.into()),
                                        op: op!("="),
                                        right: box Expr::Fn(FnExpr {
                                            ident: None,
                                            function: Function {
                                                span: DUMMY_SP,
                                                is_async: false,
                                                is_generator: false,
                                                params: vec![],
                                                body: Some(BlockStmt {
                                                    span: DUMMY_SP,
                                                    stmts: vec![Stmt::Return(ReturnStmt {
                                                        span: DUMMY_SP,
                                                        arg: Some(box quote_ident!("data").into()),
                                                    })],
                                                }),
                                                decorators: Default::default(),
                                                type_params: Default::default(),
                                                return_type: Default::default(),
                                            },
                                        }),
                                    })
                                };

                                Some(BlockStmt {
                                    span: DUMMY_SP,

                                    stmts: vec![
                                        Stmt::Decl(Decl::Var(data_decl)),
                                        Stmt::Expr(box assign_expr),
                                        Stmt::Return(ReturnStmt {
                                            span: DUMMY_SP,
                                            arg: Some(box quote_ident!("data").into()),
                                        }),
                                    ],
                                })
                            },
                            decorators: Default::default(),
                            type_params: Default::default(),
                            return_type: Default::default(),
                        },
                    })
                };

                Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: tag.as_callee(),
                    args: iter::once(
                        Expr::Call(CallExpr {
                            span: DUMMY_SP,
                            callee: tpl_obj_fn.as_callee(),
                            args: vec![],
                            type_args: Default::default(),
                        })
                        .as_arg(),
                    )
                    .chain(exprs.into_iter().map(|e| e.as_arg()))
                    .collect(),
                    type_args: Default::default(),
                })
            }

            _ => e,
        }
    }
}
