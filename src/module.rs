// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

extern crate llvm_sys as llvm;

use llvm::analysis::LLVMVerifierFailureAction;
use llvm::analysis::LLVMVerifyModule;
use llvm::bit_writer::LLVMWriteBitcodeToFD;
use llvm::bit_writer::LLVMWriteBitcodeToFile;
use llvm::core::LLVMAddFunction;
use llvm::core::LLVMBuildAlloca;
use llvm::core::LLVMBuildGlobalString;
use llvm::core::LLVMConstInt;
use llvm::core::LLVMContextCreate;
use llvm::core::LLVMContextDispose;
use llvm::core::LLVMCreateBuilderInContext;
use llvm::core::LLVMDisposeBuilder;
use llvm::core::LLVMDisposeMessage;
use llvm::core::LLVMDisposeModule;
use llvm::core::LLVMFunctionType;
use llvm::core::LLVMGetIntTypeWidth;
use llvm::core::LLVMGetTypeKind;
use llvm::core::LLVMInt32TypeInContext;
use llvm::core::LLVMInt64TypeInContext;
use llvm::core::LLVMModuleCreateWithNameInContext;
use llvm::core::LLVMPointerTypeInContext;
use llvm::core::LLVMPrintModuleToString;
use llvm::core::LLVMSetSourceFileName;
use llvm::linker::LLVMLinkModules2;
use llvm::prelude::LLVMBool;
use llvm::prelude::LLVMBuilderRef;
use llvm::prelude::LLVMContextRef;
use llvm::prelude::LLVMModuleRef;
use llvm::prelude::LLVMTypeRef;
use llvm::prelude::LLVMValueRef;
use llvm::LLVMTypeKind;

use std::collections::HashMap;
use std::env;
use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uint;
use std::ffi::c_ulonglong;
use std::ffi::CStr;
use std::fmt;
use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::ptr;

use crate::command;
use crate::exit_code;
use crate::options;

use command::Command;
use exit_code::exit;
use exit_code::ExitCode;
use options::CodeGenType;
use options::OutputType;
use options::RunOptions;

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
    pub f:              Option<LLVMValueRef>,
    pub f_sig:          Option<FunctionSignature>,
    pub module:         LLVMModuleRef,
    pub name:           &'a String,
    pub objects:        Vec<String>,
    pub scope:          Scope,
    pub t_i32:          LLVMTypeRef,
    pub t_i64:          LLVMTypeRef,
    pub t_opaque:       LLVMTypeRef,
    pub verbose:        bool,
}

impl <'a> ModuleBundle<'a> {
    pub fn new(name: &'a String, verbose: bool) -> Self {
        let bundle = unsafe {
            let context = LLVMContextCreate();
            let n = Self::value_name(name.as_str());
            let module = LLVMModuleCreateWithNameInContext(n.as_ptr() as *const c_char, context);
            let builder = LLVMCreateBuilderInContext(context);
            ModuleBundle{
                builder,
                context,
                f: None,
                f_sig: None,
                module,
                name,
                objects: Vec::new(),
                scope: Scope::new(),
                t_i32: LLVMInt32TypeInContext(context),
                t_i64: LLVMInt64TypeInContext(context),
                t_opaque: LLVMPointerTypeInContext(context, 0 as c_uint),
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
            let t_f = LLVMFunctionType(
                t_ret,
                params.as_mut_ptr(),
                params.len() as c_uint,
                is_va_arg as LLVMBool
            );
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
        let (name, result) = if f == "-" {
            let result: c_int = unsafe {
                LLVMWriteBitcodeToFD(self.module, 1, false as LLVMBool, false as LLVMBool)
            };
            ("Stdout".to_string(), result)
        } else {
            let name = format!("{}\0", f);
            let result: c_int = unsafe {
                LLVMWriteBitcodeToFile(self.module, name.as_ptr() as *const c_char)
            };
            (name, result)
        };
        if result != 0 as c_int {
            eprintln!("Failed to write module to file '{}'", name);
            exit(ExitCode::WriteError);
        }
    }

    pub fn set_sourcefile_name(&mut self, f: &str) -> () {
        unsafe {
            LLVMSetSourceFileName(self.module, f.as_ptr() as *const c_char, f.len());
        }
    }

    pub fn get_int_width(t: LLVMTypeRef) -> usize {
        let kind: LLVMTypeKind = unsafe { LLVMGetTypeKind(t) };
        if kind != LLVMTypeKind::LLVMIntegerTypeKind {
            eprintln!("Unexpected non-integer type");
            exit(ExitCode::ModuleError);
        };
        (unsafe { LLVMGetIntTypeWidth(t) }) as usize
    }

    pub fn type_name_from(t: LLVMTypeRef) -> String {
        let kind: LLVMTypeKind = unsafe { LLVMGetTypeKind(t) };
        match kind {
            LLVMTypeKind::LLVMPointerTypeKind   => "ptr",
            LLVMTypeKind::LLVMIntegerTypeKind   => {
                match Self::get_int_width(t) {
                    32  => "i32",
                    64  => "i64",
                    _   => {
                        eprintln!("Unsupported integer type width");
                        exit(ExitCode::ModuleError);
                    },
                }
            }
            _                                   => {
                eprintln!("Unsupported type kind");
                exit(ExitCode::ModuleError);
            }
        }.to_string()
    }

    /// Objects required for linking the final object file/executable should be pushed before
    /// output
    pub fn push_object(&mut self, obj_path: String) -> () {
        let path = Path::new(&obj_path);
        if !path.is_file() || ".o" == path.extension().unwrap() {
            eprintln!("Expected object file '{}'", obj_path);
            exit(ExitCode::ModuleError);
        }
        self.objects.push(obj_path);
    }

    pub fn write_module(&mut self, options: &RunOptions, output: &OutputType) -> bool {
        match *output {
            OutputType::Stdout  => {
                match options.codegen_type {
                    CodeGenType::Llvmir     => println!("{}", self),
                    CodeGenType::Bitcode    => self.write_bitcode_to_file("-"),
                    _                       => {
                        eprintln!("Unexpected  '{}' for output to stdout", options.codegen_type);
                        return false;
                    },
                }
            },
            OutputType::File(f) => {
                match options.codegen_type {
                    CodeGenType::Llvmir     => {
                        let string: String = self.to_string();
                        let mut file = match File::create(f) {
                            Ok(file)    => file,
                            Err(msg)    => {
                                eprintln!("Failed to open output file '{}': {}", f, msg);
                                return false;
                            }
                        };
                        match file.write_all(string.as_bytes()) {
                            Ok(())      => (),
                            Err(msg)    => {
                                eprintln!("Failed to write to output file '{}': {}", f, msg);
                                return false;
                            }
                        }
                    },
                    CodeGenType::Bitcode    => self.write_bitcode_to_file(f),
                    CodeGenType::Object     => {
                        let f_path = Path::new(f);
                        let temp_dir = env::temp_dir();
                        let f_stem = f_path.file_stem().unwrap().to_str().unwrap();
                        let f_bc = temp_dir.join(format!("{}.bc", f_stem));
                        let f_bc_str = f_bc.to_str().unwrap();
                        self.write_bitcode_to_file(f_bc_str);
                        self.object_file_from_bitcode(f, f_bc_str);
                    },
                    CodeGenType::Executable => {
                        let f_path = Path::new(f);
                        let temp_dir = env::temp_dir();
                        let f_stem = f_path.file_stem().unwrap().to_str().unwrap();
                        let f_bc = temp_dir.join(format!("{}.bc", f_stem));
                        let f_bc_str = f_bc.to_str().unwrap();
                        self.write_bitcode_to_file(f_bc_str);
                        let f_obj = temp_dir.join(format!("{}.obj", f_stem));
                        let f_obj_str = f_obj.to_str().unwrap();
                        self.object_file_from_bitcode(f_obj_str, f_bc_str);
                        self.executable_file_from_object(f, f_obj_str);
                    },
                    CodeGenType::Unset      => {
                        eprintln!("Cannot write module with unset codegen type");
                        exit(ExitCode::WriteError);
                    },
                }
            },
        }
        true
    }

    fn object_file_from_bitcode(&self, f_obj: &str, f_bc: &str) -> () {
        let arg_output = format!("-o={}", f_obj);
        let args: Vec<&str> = vec![
            "-relocation-model=pic",
            "-filetype=obj",
            arg_output.as_str(),
            f_bc,
        ];
        let result_llc = Command::run("llc", &args);
        if !result_llc.success {
            eprintln!("Failed to write object file '{}' from bitcode file '{}'", f_obj, f_bc);
            exit(ExitCode::WriteError);
        } else if result_llc.stdout.is_some() {
            println!("{}", result_llc.stdout.unwrap());
        }
    }

    fn executable_file_from_object(&self, f_bin: &str, f_obj: &str) -> () {
        let mut args: Vec<&str> = vec!["-o", f_bin, f_obj];
        for object in self.objects.iter() {
            args.push(object.as_str());
        }
        let result_clang = Command::run("clang", &args);
        if !result_clang.success {
            eprintln!("Failed to write executable file '{}' from object file '{}'", f_bin, f_obj);
            exit(ExitCode::WriteError);
        } else if result_clang.stdout.is_some() {
            println!("{}", result_clang.stdout.unwrap());
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
