// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

extern crate llvm_sys as llvm;
use llvm::prelude::LLVMValueRef;

pub type GenResult = Result<LLVMValueRef, &'static str>;

pub trait AstGenerator {
    fn visit(&mut self, ast: &dyn Ast) -> GenResult;
}

pub trait AstVisitor {
    fn visit(&mut self, ast: &dyn Ast) -> bool;
}

pub trait Ast {
    fn accept(&self, visitor: &mut dyn AstVisitor) -> bool;
    fn accept_gen(&self, visitor: &mut dyn AstGenerator) -> GenResult;
    fn is_expr(&self) -> bool;
    fn get_expr(&self) -> &ExprKind;
    fn get_vars(&self) -> usize;
    fn to_string(&self) -> String;
}

#[derive(Clone)]
pub enum Factor {
    Ident(String),
    Number(i64),
}

pub fn factor_to_string(f: &Factor) -> String {
    match f {
        Factor::Ident(s)    => format!("Ident({})", s),
        Factor::Number(n)   => format!("{}", n),
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
    vars: usize,
}

impl <'a> Expr<'a> {
    pub fn new(expr: ExprKind<'a>, n: usize) -> Self {
        Expr{expr: expr, vars: n}
    }

    pub fn new_number(n: i64) -> Self {
        Expr::new(ExprKind::Factor(Factor::Number(n)), 0)
    }

    pub fn new_ident(text: String) -> Self {
        Expr::new(ExprKind::Factor(Factor::Ident(text)), 0)
    }

    pub fn new_binop(op: Operator, e_left: &'a Expr<'a>, e_right: &'a Expr<'a>) -> Self {
        Expr::new(ExprKind::BinaryOp(op, e_left, e_right), e_left.vars + e_right.vars)
    }

    pub fn new_withdecl(vars: Vars, e: &'a Expr<'a>) -> Self {
        let n = vars.len();
        Expr::new(ExprKind::WithDecl(vars, e), n + e.vars)
    }
}

impl <'a> Ast for Expr<'a> {
    fn accept(&self, visitor: &mut dyn AstVisitor) -> bool {
        visitor.visit(self)
    }

    fn accept_gen(&self, visitor: &mut dyn AstGenerator) -> GenResult {
        visitor.visit(self)
    }

    fn is_expr(&self) -> bool {
        true
    }

    fn get_expr(&self) -> &ExprKind {
        &self.expr    
    }

    fn get_vars(&self) -> usize {
        self.vars
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
        Expr::new(Default::default(), Default::default())
    }
}
