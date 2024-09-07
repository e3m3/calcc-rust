// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashSet;

use crate::ast;
use crate::exit_code;
use crate::options;

use ast::Ast;
use ast::AstVisitor;
use ast::Expr;
use ast::ExprKind;
use ast::Factor;
use ast::Operator;
use ast::Vars;
use exit_code::exit;
use exit_code::ExitCode;
use options::RunOptions;

pub struct Scope {
    vars: HashSet<String>, 
}

impl Scope {
    pub fn new() -> Self {
        Scope{vars: Default::default()}
    }

    pub fn add_var(&mut self, var: &String, options: &RunOptions) -> bool {
        let result = self.vars.insert(var.clone());
        if options.verbose && result {
            eprintln!("Added var '{}' to scope", *var);
        }
		result
    }

    pub fn contains_var(&self, var: &String, options: &RunOptions) -> bool {
        let result = self.vars.contains(var);
        if options.verbose && result {
            eprintln!("Found var '{}' in scope", var);
        }
        if !result {
            eprintln!("Found unbound var '{}' in scope", var);
        }
        result
    }
}

struct DeclCheck<'a> {
    scope: Scope,
    options: &'a RunOptions,
}

impl <'a> DeclCheck<'a> {
    pub fn new(options: &'a RunOptions) -> Self {
        DeclCheck{scope: Scope::new(), options: options}
    }

    pub fn check_expr_undefined(&self) -> bool {
        false
    }

    pub fn check_expr_factor(&self, f: &Factor) -> bool {
        match f {
            Factor::Number(_)   => true,
            Factor::Ident(var)  => self.scope.contains_var(&var, self.options),
        }
    }

    pub fn check_expr_binop(&mut self, _op: &Operator, e_left: &Expr, e_right: &Expr) -> bool {
        self.visit(e_left) && self.visit(e_right)
    }

    pub fn check_expr_withdecl(&mut self, vars: &Vars, e: &Expr) -> bool {
        for var in vars {
            let result = self.scope.add_var(var, self.options);
			if !result {
				eprintln!("Tried to declare variable {} more than once", var);
				return false;
			};
        }
        self.visit(e)
    }
}

impl <'a> AstVisitor for DeclCheck<'a> {
    fn visit(&mut self, ast: &dyn Ast) -> bool {
        if ast.is_expr() {
            let expr: &ExprKind = ast.get_expr();
            return match expr {
                ExprKind::Undefined                     => self.check_expr_undefined(),
                ExprKind::Factor(f)                     => self.check_expr_factor(f),
                ExprKind::BinaryOp(op, e_left, e_right) => self.check_expr_binop(op, e_left, e_right),
                ExprKind::WithDecl(vars, e)             => self.check_expr_withdecl(vars, e),
            }
        }
        false
    }
}

struct ReprCheck<'a> {
    options: &'a RunOptions,
}

impl <'a> ReprCheck<'a> {
    pub fn new(options: &'a RunOptions) -> Self {
        ReprCheck{options: options}
    }

    pub fn check_expr_undefined(&self) -> bool {
        false
    }

    pub fn check_expr_factor(&self, f: &Factor) -> bool {
        match f {
            Factor::Ident(_)        => true,
            Factor::Number(n)       => {
                let result: bool = *n <= std::i64::MAX && *n >= std::i64::MIN;
                if self.options.verbose && result {
                    eprintln!("Number '{}' passed repr check", *n);
                } else if !result {
                    eprintln!("Number '{}' failed repr check", *n);
                }
                result
            }
        }
    }

    pub fn check_expr_binop(&mut self, _op: &Operator, e_left: &Expr, e_right: &Expr) -> bool {
        self.visit(e_left) && self.visit(e_right)
    }

    pub fn check_expr_withdecl(&mut self, _vars: &Vars, e: &Expr) -> bool {
        self.visit(e)
    }
}

impl <'a> AstVisitor for ReprCheck<'a> {
    fn visit(&mut self, ast: &dyn Ast) -> bool {
        if ast.is_expr() {
            let expr: &ExprKind = ast.get_expr();
            return match expr {
                ExprKind::Undefined                     => self.check_expr_undefined(),
                ExprKind::Factor(f)                     => self.check_expr_factor(f),
                ExprKind::BinaryOp(op, e_left, e_right) => self.check_expr_binop(op, e_left, e_right),
                ExprKind::WithDecl(vars, e)             => self.check_expr_withdecl(vars, e),
            }
        }
        false
    }
}

pub struct Semantics {}

impl Semantics {
    pub fn check_all(ast: &dyn Ast, options: &RunOptions) -> bool {
        let mut decl_check: DeclCheck = DeclCheck::new(options);
        let decl_result: bool = ast.accept(&mut decl_check);
        if !decl_result {
            eprintln!("AST failed DeclCheck semantics check");
            exit(ExitCode::SemanticError);
        }
        let mut repr_check: ReprCheck = ReprCheck::new(options);
        let repr_result: bool = ast.accept(&mut repr_check);
        if !repr_result {
            eprintln!("AST failed ReprCheck semantics check");
            exit(ExitCode::SemanticError);
        }
        if options.sem_exit { exit(ExitCode::Ok); }
        decl_result && repr_result
    }
}
