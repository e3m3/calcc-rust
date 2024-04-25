// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

use crate::sem;
use sem::AstVisitor;

pub trait Ast {
    fn accept(&self, visitor: &mut dyn AstVisitor) -> bool;
    fn is_expr(&self) -> bool;
    fn get_expr(&self) -> &ExprKind;
    fn to_string(&self) -> String;
}

#[derive(Clone)]
pub enum Factor {
    Ident(String),
    Number(String),
}

pub fn factor_to_string(f: &Factor) -> String {
    match f {
        Factor::Ident(s)    => format!("Ident({})", s),
        Factor::Number(s)   => s.clone(),
    }
}

#[derive(Clone,Copy)]
pub enum Operator {
    Add,
    Div,
    Mul,
    Sub,
}

pub fn op_to_string(op: &Operator) -> String {
    match op {
        Operator::Add   => String::from("Add"),
        Operator::Div   => String::from("Div"),
        Operator::Mul   => String::from("Mul"),
        Operator::Sub   => String::from("Sub"),
    }
}

pub type Vars = Vec<String>;

#[derive(Clone)]
pub enum ExprKind<'a> {
    Undefined,
    Factor(Factor),
    BinaryOp(Operator, &'a Expr<'a>, &'a Expr<'a>),
    WithDecl(Vars, &'a Expr<'a>),
}

pub fn vars_to_string(vars: &Vars) -> String {
    format!("Vars([{}])", vars.join(","))
}

impl <'a> Default for ExprKind<'a> {
    fn default() -> Self {
        ExprKind::Undefined
    }
}

pub struct Expr<'a> {
    expr: ExprKind<'a>,
}

impl <'a> Expr<'a> {
    pub fn new(expr: ExprKind<'a>) -> Self {
        Expr{expr: expr}
    }

    pub fn new_number(text: String) -> Self {
        Expr::new(ExprKind::Factor(Factor::Number(text)))
    }

    pub fn new_ident(text: String) -> Self {
        Expr::new(ExprKind::Factor(Factor::Ident(text)))
    }

    pub fn new_binop(op: Operator, e_left: &'a Expr<'a>, e_right: &'a Expr<'a>) -> Self {
        Expr::new(ExprKind::BinaryOp(op, e_left, e_right))
    }

    pub fn new_withdecl(vars: Vars, e: &'a Expr<'a>) -> Self {
        Expr::new(ExprKind::WithDecl(vars, e))
    }
}

impl <'a> Ast for Expr<'a> {
    fn accept(&self, visitor: &mut dyn AstVisitor) -> bool {
        visitor.visit(self)
    }

    fn is_expr(&self) -> bool {
        true
    }

    fn get_expr(&self) -> &ExprKind {
        &self.expr    
    }

    fn to_string(&self) -> String {
        match &self.expr {
            ExprKind::Undefined                     => String::from("Undefined()"),
            ExprKind::Factor(f)                     => factor_to_string(f),
            ExprKind::BinaryOp(op, e_left, e_right) => {
                format!("BinaryOp({},{},{})", op_to_string(op), e_left.to_string(), e_right.to_string())
            }
            ExprKind::WithDecl(vars, e)             => {
                format!("WithDecl({},{})", vars_to_string(vars), e.to_string())
            }
        }
    }
}

impl <'a> Default for Expr<'a> {
    fn default() -> Self {
        Expr::new(Default::default())
    }
}
