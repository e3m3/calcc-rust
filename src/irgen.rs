extern crate llvm_sys as llvm;

use llvm::core::*;
use llvm::prelude::LLVMBasicBlockRef;
use llvm::prelude::LLVMBool;
use llvm::prelude::LLVMBuilderRef;
use llvm::prelude::LLVMContextRef;
use llvm::prelude::LLVMModuleRef;
use llvm::prelude::LLVMTypeRef;
use llvm::prelude::LLVMValueRef;

use std::collections::HashMap;
use std::ffi::c_char;
use std::ffi::c_ulonglong;
use std::ptr;
use std::str::FromStr;

use crate::ast;
use crate::exit_code;
use crate::options;

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
use options::RunOptions;

pub struct Scope {
    value_idx:  i32,
    vars:       HashMap<String, LLVMValueRef>,
}

impl Scope {
    pub fn new() -> Self {
        Scope{
            value_idx:  0,
            vars:       Default::default(),
        }
    }

    pub fn add_var(&mut self, var: &String, value: LLVMValueRef, options: &RunOptions) {
        let result = self.vars.insert(var.clone(), value);
        if options.verbose && result.is_none() {
            println!("Added var '{}' to scope", *var);
        }
        assert!(result.is_none())
    }

    pub fn clear(&mut self) {
        self.vars.clear()
    }

    pub fn get_value(&self, var: &String, options: &RunOptions) -> LLVMValueRef {
        match self.vars.get(var) {
            Some(value) => {
                if options.verbose {
                    println!("Found var '{}' in scope", var);
                };
                value.clone()
            },
            None        => {
                eprintln!("Unexpected unbound var '{}' in scope", var);
                exit(ExitCode::IRGenError);
            },
        }
    }

    pub fn next_value_name(&mut self) -> String {
        let name = String::from(format!("v{}\0", self.value_idx));
        self.value_idx += 1;
        name
    }
}

pub struct ModuleBundle<'a> {
    builder:    LLVMBuilderRef,
    context:    LLVMContextRef,
    module:     LLVMModuleRef,
    name:       &'a String,
    scope:      Scope,
    t_i64:      LLVMTypeRef,
}

impl <'a> ModuleBundle<'a> {
    pub fn new(name: &'a String) -> Self {
        let bundle = unsafe {
            let c = LLVMContextCreate();
            let n = String::new() + name + "\0";
            let m = LLVMModuleCreateWithName(n.as_ptr() as *const c_char);
            let b = LLVMCreateBuilderInContext(c);
            ModuleBundle{
                builder: b,
                context: c,
                module: m,
                name: name,
                scope: Scope::new(),
                t_i64: LLVMInt64TypeInContext(c),
            }
        };
        bundle
    }
}

impl <'a> Drop for ModuleBundle<'a> {
    fn drop(&mut self) {
        self.scope.clear();
        unsafe {
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }
}

pub struct IRGen<'a, 'b> {
    bundle:     &'a mut ModuleBundle<'b>,
    options:    &'a RunOptions,
}

impl <'a, 'b> IRGen<'a, 'b> {
    pub fn new(bundle: &'a mut ModuleBundle<'b>, options: &'a RunOptions) -> Self {
        IRGen{ 
            bundle:     bundle,
            options:    options,
        }
    }

    pub fn gen_entry(&mut self) -> LLVMBasicBlockRef {
        unsafe {
            let f_type = LLVMFunctionType(self.bundle.t_i64, ptr::null_mut(), 0, 0);
            let f_name = String::new() + self.bundle.name + "_main\0";
            let f = LLVMAddFunction(self.bundle.module, f_name.as_ptr() as *const c_char, f_type);
            let bb = LLVMAppendBasicBlockInContext(
                self.bundle.context,
                f,
                String::from("entry").as_ptr() as *const i8,
            );
            LLVMPositionBuilderAtEnd(self.bundle.builder, bb);
            bb
        }
    }

    pub fn gen_ret(&mut self, bb: LLVMBasicBlockRef, ret_value: LLVMValueRef) {
        unsafe {
            LLVMPositionBuilderAtEnd(self.bundle.builder, bb);
            LLVMBuildRet(self.bundle.builder, ret_value);
        };
    }

    pub fn gen_expr_undefined(&mut self) -> GenResult {
        Err("AST contained unexpected undefined expression")
    }

    pub fn gen_expr_factor(&mut self, f: &Factor) -> GenResult {
        let value = match f {
            Factor::Ident(name)     => {
                let alloca_value = self.bundle.scope.get_value(name, self.options);
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
            Factor::Number(text)    => {
                let result = if Self::is_hex_number(text) {
                    i64::from_str_radix(&text[2..], 16)
                } else {
                    i64::from_str(text.as_str())
                };
                self.get_constint(result.unwrap())
            },
        };
        Ok(value)
    }

    pub fn gen_expr_binop(&mut self, op: &Operator, e_left: &Expr, e_right: &Expr) -> GenResult {
        let value_left = self.visit(e_left).unwrap();
        let value_right = self.visit(e_right).unwrap();
        let value_name = self.bundle.scope.next_value_name();
        let value_name_c = value_name.as_ptr() as *const c_char;
        let value = match op {
            Operator::Add   => unsafe {
                LLVMBuildAdd(self.bundle.builder, value_left, value_right, value_name_c)
            },
            Operator::Div   => unsafe {
                LLVMBuildSDiv(self.bundle.builder, value_left, value_right, value_name_c)
            },
            Operator::Mul   => unsafe {
                LLVMBuildMul(self.bundle.builder, value_left, value_right, value_name_c)
            },
            Operator::Sub   => unsafe {
                LLVMBuildSub(self.bundle.builder, value_left, value_right, value_name_c)
            },
        };
        Ok(value)
    }

    pub fn gen_expr_withdecl(&mut self, vars: &Vars, e: &Expr) -> GenResult {
        for var in vars {
            let alloca_value = unsafe {
                let v = String::new() + var + "\0";
                LLVMBuildAlloca(self.bundle.builder, self.bundle.t_i64, v.as_ptr() as *const c_char)
            };
            let init_value = self.get_constint(0);
            let store_value = unsafe {
                LLVMBuildStore(self.bundle.builder, init_value, alloca_value)
            };
            self.bundle.scope.add_var(var, alloca_value, self.options);
        };
        self.visit(e)
    }

    pub fn gen(ast: &dyn Ast, bundle: &'a mut ModuleBundle<'b>, options: &'a RunOptions) -> bool {
        let mut ir_gen = IRGen::new(bundle, options);
        let bb = ir_gen.gen_entry();
        let ir_gen_result: GenResult = ast.accept_gen(&mut ir_gen);
        let ir_gen_value: LLVMValueRef = match ir_gen_result {
            Ok(value)   => value,
            Err(msg)    => {
                eprintln!("{}\nAST failed IR code generation", msg);
                exit(ExitCode::IRGenError);
            },
        };
        ir_gen.gen_ret(bb, ir_gen_value);
        if options.print_ir { unsafe {
            LLVMDumpModule(bundle.module)
        }}
        true
    }

    fn get_constint(&self, num: i64) -> LLVMValueRef {
        let c = num as c_ulonglong;
        let do_sext = true as LLVMBool;
        unsafe {
            LLVMConstInt(self.bundle.t_i64, c, do_sext)
        }
    }

    fn is_hex_number(text: &String) -> bool {
        text.len() >= 2 && "0x" == &text[0..2]
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
