// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

extern crate llvm_sys as llvm;

use llvm::core::*;
use llvm::analysis::LLVMVerifierFailureAction;
use llvm::analysis::LLVMVerifyModule;
use llvm::bit_writer::LLVMWriteBitcodeToFile;
use llvm::linker::LLVMLinkModules2;
use llvm::prelude::LLVMBool;
use llvm::prelude::LLVMBuilderRef;
use llvm::prelude::LLVMContextRef;
use llvm::prelude::LLVMModuleRef;
use llvm::prelude::LLVMTypeRef;
use llvm::prelude::LLVMValueRef;

use std::collections::HashMap;
use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uint;
use std::ffi::c_ulonglong;
use std::ffi::CStr;
use std::fmt;
use std::fmt::Display;
use std::ptr;

use crate::exit_code;

use exit_code::exit;
use exit_code::ExitCode;

#[derive(Clone)]
pub struct FunctionSignature {
    pub t_ret:  LLVMTypeRef,
    pub params: Vec<LLVMTypeRef>,
}

impl FunctionSignature {
    pub fn new(t_ret: LLVMTypeRef, params: Vec<LLVMTypeRef>) -> Self {
        FunctionSignature{t_ret, params}
    }
}

pub struct ModuleBundle<'a> {
    pub builder:        LLVMBuilderRef,
    pub context:        LLVMContextRef,
    pub module:         LLVMModuleRef,
    pub name:           &'a String,
    pub scope:          Scope,
    pub t_i32:          LLVMTypeRef,
    pub t_i64:          LLVMTypeRef,
    pub t_opaque:       LLVMTypeRef,
    pub f:              Option<LLVMValueRef>,
    pub f_sig:          Option<FunctionSignature>,
    pub verbose:        bool,
}

impl <'a> ModuleBundle<'a> {
    pub fn new(name: &'a String, verbose: bool) -> Self {
        let bundle = unsafe {
            let c = LLVMContextCreate();
            let n = Self::value_name(name.as_str());
            let m = LLVMModuleCreateWithNameInContext(n.as_ptr() as *const c_char, c);
            let b = LLVMCreateBuilderInContext(c);
            ModuleBundle{
                builder: b,
                context: c,
                module: m,
                name,
                scope: Scope::new(),
                t_i32: LLVMInt32TypeInContext(c),
                t_i64: LLVMInt64TypeInContext(c),
                t_opaque: LLVMPointerTypeInContext(c, 0 as c_uint),
                f: None,
                f_sig: None,
                verbose,
            }
        };
        bundle
    }

    pub fn declare_global_string(&mut self, name: &str, string: &str) -> LLVMValueRef {
        let n = Self::value_name(name);
        let s = String::from(string);
        let value: LLVMValueRef = unsafe {
            LLVMBuildGlobalString(self.builder, s.as_ptr() as *const c_char, n.as_ptr() as *const c_char)
        };
        self.insert_value(&n, value);
        value
    }

    pub fn emit_declaration(
        &mut self,
        name: &String,
        t_ret: LLVMTypeRef,
        params: &mut Vec<LLVMTypeRef>,
        is_va_arg: bool,
    ) -> LLVMValueRef {
        let value = unsafe {
            let t_f = LLVMFunctionType(t_ret, params.as_mut_ptr(), params.len() as c_uint, is_va_arg as LLVMBool);
            LLVMAddFunction(self.module, name.as_ptr() as *const c_char, t_f)
        };
        self.insert_value(name, value);
        value
    }

    pub fn gen_alloca(&mut self, name: &str, t: LLVMTypeRef) -> LLVMValueRef {
        let n = Self::value_name(name);
        let value = unsafe {
            LLVMBuildAlloca(self.builder, t, n.as_ptr() as *const c_char)
        };
        self.insert_value(&n, value);
        value
    }

    pub fn get_constint(&self, t: LLVMTypeRef, num: i64) -> LLVMValueRef {
        let c = num as c_ulonglong;
        let do_sext = true as LLVMBool;
        unsafe {
            LLVMConstInt(t, c, do_sext)
        }
    }

    pub fn get_value(&mut self, name: &String) -> LLVMValueRef {
        self.scope.get_value(name, self.verbose)
    }

    pub fn insert_value(&mut self, name: &String, value: LLVMValueRef) -> () {
        let result = self.scope.add_var(name, value, self.verbose);
        if result.is_some() {
            eprintln!("Tried to declare value {} more than once", name);
            exit(ExitCode::ModuleError);
        }
    }

    pub fn link_into(&mut self, other: &mut ModuleBundle) -> bool {
        let result: LLVMBool = unsafe { LLVMLinkModules2 (
            self.module,
            other.module,
        )};
        result == false as LLVMBool
    }

    pub fn value_name(s: &str) -> String {
        String::from(s) + "\0"
    }

    pub fn verify_module(&self) -> bool {
        let mut error_ptr: *mut c_char = ptr::null_mut();
        let result: LLVMBool = unsafe {
            LLVMVerifyModule(
                self.module,
                LLVMVerifierFailureAction::LLVMReturnStatusAction,
                &mut error_ptr as *mut *mut c_char,
            )
        };
        unsafe {
            let c_string = CStr::from_ptr(error_ptr as *const c_char);
            let s = c_string.to_str().expect("Unable to read module verification error string");
            if !s.is_empty() {
                eprintln!("{}", s);
            }
            LLVMDisposeMessage(error_ptr);
        }
        result == false as LLVMBool
    }

    pub fn write_bitcode_to_file(&mut self, f: &str) -> () {
        let name = String::from(f) + "\0";
        let result: c_int = unsafe { LLVMWriteBitcodeToFile(self.module, name.as_ptr() as *const c_char) };
        if result != 0 as c_int {
            eprintln!("Failed to write module to file '{}'", name);
            exit(ExitCode::WriteError);
        }
    }
}

impl <'a> Display for ModuleBundle<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let c_string_ptr: *mut c_char = unsafe {
            LLVMPrintModuleToString(self.module)
        };
        let c_string: &CStr = unsafe {
            CStr::from_ptr(c_string_ptr)
        };
        let string: String = String::from(c_string.to_str().unwrap());
        unsafe {
            LLVMDisposeMessage(c_string_ptr)
        };
        write!(f, "{}", string)
    }
}


impl <'a> Drop for ModuleBundle<'a> {
    fn drop(&mut self) -> () {
        self.scope.clear();
        unsafe {
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }
}

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

    pub fn add_var(&mut self, var: &String, value: LLVMValueRef, verbose: bool) -> Option<LLVMValueRef> {
        let result = self.vars.insert(var.clone(), value);
        if verbose && result.is_none() {
            eprintln!("Added var '{}' to scope", *var);
        }
        result
    }

    pub fn clear(&mut self) {
        self.vars.clear()
    }

    pub fn get_value(&self, var: &String, verbose: bool) -> LLVMValueRef {
        match self.vars.get(var) {
            Some(value) => {
                if verbose {
                    eprintln!("Found var '{}' in scope", var);
                };
                *value
            },
            None        => {
                eprintln!("Unexpected unbound var '{}' in scope", var);
                exit(ExitCode::ModuleError);
            },
        }
    }

    pub fn next_value_name(&mut self) -> String {
        let name = format!("v{}\0", self.value_idx);
        self.value_idx += 1;
        name
    }
}
