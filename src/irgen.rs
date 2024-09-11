// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

extern crate llvm_sys as llvm;

use llvm::core::*;
use llvm::prelude::LLVMBasicBlockRef;
use llvm::prelude::LLVMBool;
use llvm::prelude::LLVMTypeRef;
use llvm::prelude::LLVMValueRef;

use std::ffi::c_char;
use std::ffi::c_uint;

use crate::ast;
use crate::exit_code;
use crate::module;

use ast::Ast;
use ast::AstGenerator;
use ast::Expr;
use ast::ExprKind;
use ast::Factor;
use ast::GenResult;
use ast::Operator;
use ast::Vars;
use exit_code::exit;
use exit_code::ExitCode;
use module::FunctionSignature;
use module::ModuleBundle;

pub struct IRGen<'a, 'b> {
    bundle:         &'a mut ModuleBundle<'b>,
}

impl <'a, 'b> IRGen<'a, 'b> {
    pub fn new(bundle: &'a mut ModuleBundle<'b>) -> Self {
        IRGen{bundle}
    }

    fn gen_entry(&mut self, n: usize) -> LLVMBasicBlockRef {
        let f_name = String::new() + self.bundle.name + "_main\0";
        let mut param_types: Vec<LLVMTypeRef> = Vec::new();
        for _ in 0..n {
            param_types.push(self.bundle.t_i64);
        };
        unsafe {
            let t_ret = self.bundle.t_i64;
            let f_type = LLVMFunctionType(
                t_ret,
                param_types.as_mut_ptr(),
                n as u32,
                false as LLVMBool
            );
            let f = LLVMAddFunction(self.bundle.module, f_name.as_ptr() as *const c_char, f_type);
            self.bundle.f = Some(f);
            self.bundle.f_sig = Some(FunctionSignature::new(t_ret, param_types));
            let bb = LLVMAppendBasicBlockInContext(
                self.bundle.context,
                f,
                ModuleBundle::value_name("entry").as_ptr() as *const c_char,
            );
            LLVMPositionBuilderAtEnd(self.bundle.builder, bb);
            bb
        }
    }

    fn gen_ret(&mut self, bb: LLVMBasicBlockRef, ret_value: LLVMValueRef) {
        unsafe {
            LLVMPositionBuilderAtEnd(self.bundle.builder, bb);
            LLVMBuildRet(self.bundle.builder, ret_value);
        };
    }

    fn gen_expr_undefined(&mut self) -> GenResult {
        Err("AST contained unexpected undefined expression")
    }

    fn gen_expr_factor(&mut self, f: &Factor) -> GenResult {
        let value = match f {
            Factor::Number(n)   => self.bundle.get_constint(self.bundle.t_i64, *n),
            Factor::Ident(name) => {
                let n = ModuleBundle::value_name(name);
                let alloca_value = self.bundle.get_value(&n);
                let value_name = self.bundle.scope.next_value_name();
                unsafe {
                    LLVMBuildLoad2(
                        self.bundle.builder,
                        self.bundle.t_i64,
                        alloca_value,
                        value_name.as_ptr() as *const c_char
                    )
                }
            },
        };
        Ok(value)
    }

    fn gen_expr_binop(&mut self, op: &Operator, e_left: &Expr, e_right: &Expr) -> GenResult {
        let value_left = self.visit(e_left).unwrap();
        let value_right = self.visit(e_right).unwrap();
        let value_name = self.bundle.scope.next_value_name();
        let value_name_c = value_name.as_ptr() as *const c_char;
        let value = match op {
            Operator::Add   => unsafe {
                LLVMBuildNSWAdd(self.bundle.builder, value_left, value_right, value_name_c)
            },
            Operator::Div   => unsafe {
                LLVMBuildSDiv(self.bundle.builder, value_left, value_right, value_name_c)
            },
            Operator::Mul   => unsafe {
                LLVMBuildNSWMul(self.bundle.builder, value_left, value_right, value_name_c)
            },
            Operator::Sub   => unsafe {
                LLVMBuildNSWSub(self.bundle.builder, value_left, value_right, value_name_c)
            },
        };
        Ok(value)
    }

    /// Generate the LLVM IR for the given WithDecl expression.
    /// Each named variable is assumed to reference the corresponding function parameter,
    /// in the order they appear (e.g., "with a,b" maps to "foo(%0, %1)", where %0 is a and %1 is b).
    /// Note that this only works for simple single statement programs; a base offset would be
    /// needed if longer multi-statement programs are implemented.
    fn gen_expr_withdecl(&mut self, vars: &Vars, e: &Expr) -> GenResult {
        let f = self.bundle.f.expect("Missing parent function");
        for (i, var) in vars.iter().enumerate() {
            unsafe {
                let alloca_value = self.bundle.gen_alloca(var.as_str(), self.bundle.t_i64);
                let init_value = LLVMGetParam(f, i as c_uint);
                let _store_value = LLVMBuildStore(self.bundle.builder, init_value, alloca_value);
            };
        };
        self.visit(e)
    }

    pub fn gen(ast: &dyn Ast, bundle: &'a mut ModuleBundle<'b>) -> bool {
        let mut ir_gen = IRGen::new(bundle);
        let n = ast.get_vars();
        let bb = ir_gen.gen_entry(n);
        let ir_gen_result: GenResult = ast.accept_gen(&mut ir_gen);
        let ir_gen_value: LLVMValueRef = match ir_gen_result {
            Ok(value)   => value,
            Err(msg)    => {
                eprintln!("{}\nAST failed IR code generation", msg);
                exit(ExitCode::IRGenError);
            },
        };
        ir_gen.gen_ret(bb, ir_gen_value);
        true
    }
}

impl <'a, 'b> AstGenerator for IRGen<'a, 'b> {
    fn visit(&mut self, ast: &dyn Ast) -> GenResult {
        if ast.is_expr() {
            let expr: &ExprKind = ast.get_expr();
            let result = match expr {
                ExprKind::Undefined                     => self.gen_expr_undefined(),
                ExprKind::Factor(f)                     => self.gen_expr_factor(f),
                ExprKind::BinaryOp(op, e_left, e_right) => self.gen_expr_binop(op, e_left, e_right),
                ExprKind::WithDecl(vars, e)             => self.gen_expr_withdecl(vars, e),
            };
            return result;
        }
        Err("AST for IR generator is not an expression")
    }
}
