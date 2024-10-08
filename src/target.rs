// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

extern crate llvm_sys as llvm;

use llvm::error::LLVMErrorRef;
use llvm::error::LLVMDisposeErrorMessage;
use llvm::error::LLVMGetErrorMessage;
use llvm::core::LLVMDisposeMessage;
use llvm::core::LLVMSetTarget;
use llvm::target::LLVM_InitializeAllAsmParsers;
use llvm::target::LLVM_InitializeAllAsmPrinters;
use llvm::target::LLVM_InitializeAllTargetInfos;
use llvm::target::LLVM_InitializeAllTargetMCs;
use llvm::target::LLVM_InitializeAllTargets;
use llvm::target::LLVMDisposeTargetData;
use llvm::target::LLVMSetModuleDataLayout;
use llvm::target::LLVMTargetDataRef;
use llvm::target_machine::LLVMCreateTargetDataLayout;
use llvm::target_machine::LLVMCreateTargetMachineOptions;
use llvm::target_machine::LLVMCreateTargetMachineWithOptions;
use llvm::target_machine::LLVMCodeGenOptLevel;
use llvm::target_machine::LLVMCodeModel;
use llvm::target_machine::LLVMDisposeTargetMachine;
use llvm::target_machine::LLVMDisposeTargetMachineOptions;
use llvm::target_machine::LLVMGetDefaultTargetTriple;
use llvm::target_machine::LLVMGetFirstTarget;
use llvm::target_machine::LLVMGetTargetFromTriple;
use llvm::target_machine::LLVMRelocMode;
use llvm::target_machine::LLVMTargetMachineOptionsRef;
use llvm::target_machine::LLVMTargetMachineOptionsSetCodeGenOptLevel;
use llvm::target_machine::LLVMTargetMachineOptionsSetCodeModel;
use llvm::target_machine::LLVMTargetMachineOptionsSetRelocMode;
use llvm::target_machine::LLVMTargetMachineRef;
use llvm::target_machine::LLVMTargetRef;

use llvm::transforms::pass_builder::*;

use std::ffi::c_char;
use std::ffi::CStr;
use std::ptr;

use crate::exit_code;
use crate::module;
use crate::options;

use exit_code::exit;
use exit_code::ExitCode;
use module::ModuleBundle;
use options::OptLevel;

static mut INITIALIZED: bool = false;

pub type TargetString = *mut c_char;

pub struct Target {
    pub string: TargetString,
    pub target: LLVMTargetRef,
}

impl Target {
    pub fn new() -> Self {
        Self::init();
        let string: TargetString = unsafe { LLVMGetDefaultTargetTriple() };
        if string.is_null() {
            eprintln!("Failed to get default target string");
            exit(ExitCode::TargetError);
        }
        let mut target: LLVMTargetRef = unsafe { LLVMGetFirstTarget() };
        let mut error_ptr: *mut c_char = ptr::null_mut();
        unsafe {
            LLVMGetTargetFromTriple(
                string,
                &mut target as *mut LLVMTargetRef,
                &mut error_ptr as *mut *mut c_char
            );
            if !error_ptr.is_null() {
                let c_string = CStr::from_ptr(error_ptr as *const c_char);
                let s = c_string.to_str().expect("Unable to read target triple error string");
                if !s.is_empty() {
                    eprintln!("{}", s);
                    LLVMDisposeMessage(error_ptr);
                    exit(ExitCode::TargetError);
                } else {
                    LLVMDisposeMessage(error_ptr);
                }
            }
        }
        if target.is_null() {
            eprintln!("Failed to lookup target for target string '{}'", Self::string_from(&string));
            exit(ExitCode::TargetError);
        }
        Target{string, target}
    }

    pub fn get_string(&self) -> String {
        Self::string_from(&self.string)
    }

    pub fn string_from(string: &TargetString) -> String {
        let c_str = unsafe { CStr::from_ptr(*string) };
        let result = match c_str.to_str() {
            Ok(s)       => s,
            Err(msg)    => {
                eprintln!("Failed to convert target string to c-string: {}", msg);
                exit(ExitCode::TargetError);
            }
        };
        String::from(result)
    }

    fn init() -> () {
        unsafe {
            if !INITIALIZED {
                LLVM_InitializeAllTargets();
                LLVM_InitializeAllTargetInfos();
                LLVM_InitializeAllTargetMCs();
                LLVM_InitializeAllAsmParsers();
                LLVM_InitializeAllAsmPrinters();
            };
            INITIALIZED = true;
        };
    }
}

impl Drop for Target {
    fn drop(&mut self) -> () {
        unsafe { LLVMDisposeMessage(self.string); }
    }
}

pub struct TargetMachine<'a> {
    data_layout: LLVMTargetDataRef,
    machine: LLVMTargetMachineRef,
    machine_options: LLVMTargetMachineOptionsRef,
    target: &'a mut Target,
}

impl <'a> TargetMachine<'a> {
    pub fn new(target: &'a mut Target, opt_level: OptLevel) -> Self {
        let machine_options: LLVMTargetMachineOptionsRef = unsafe { LLVMCreateTargetMachineOptions() };
        unsafe {
            LLVMTargetMachineOptionsSetCodeModel(machine_options, LLVMCodeModel::LLVMCodeModelDefault);
            LLVMTargetMachineOptionsSetRelocMode(machine_options, LLVMRelocMode::LLVMRelocPIC);
            LLVMTargetMachineOptionsSetCodeGenOptLevel(machine_options, match opt_level {
                OptLevel::O0    => LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
                OptLevel::O1    => LLVMCodeGenOptLevel::LLVMCodeGenLevelLess,
                OptLevel::O2    => LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
                OptLevel::O3    => LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
            });
        };
        let machine = unsafe { LLVMCreateTargetMachineWithOptions(
            target.target, 
            target.string,
            machine_options,
        )};
        let data_layout = unsafe { LLVMCreateTargetDataLayout(machine) };
        TargetMachine{data_layout, machine, machine_options, target}
    }
}

impl <'a> Drop for TargetMachine<'a> {
    fn drop(&mut self) -> () {
        unsafe {
            LLVMDisposeTargetData(self.data_layout);
            LLVMDisposeTargetMachineOptions(self.machine_options);
            LLVMDisposeTargetMachine(self.machine);
        };
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum Passes {
    Default(OptLevel),
    Pipeline(String),
}

pub struct PassBuilder {
    builder: LLVMPassBuilderOptionsRef,
}

impl PassBuilder {
    pub fn new() -> Self {
        let builder = unsafe { LLVMCreatePassBuilderOptions() };
        PassBuilder{builder}
    }

    pub fn run(
        &mut self,
        bundle: &mut ModuleBundle,
        machine: &mut TargetMachine,
        passes: Passes,
        no_target: bool,
    ) -> bool {
        let passes_str = match passes {
            Passes::Default(opt_level)  => format!("default<{}>\0", match opt_level {
                OptLevel::O0    => "O0",
                OptLevel::O1    => "O1",
                OptLevel::O2    => "O2",
                OptLevel::O3    => "O3",
            }),
            Passes::Pipeline(string)    => format!("{}\0", string),
        };
        unsafe {
            if !no_target {
                LLVMSetModuleDataLayout(bundle.module, machine.data_layout);
                LLVMSetTarget(bundle.module, machine.target.string);
            };
            let error: LLVMErrorRef = LLVMRunPasses(
                bundle.module,
                passes_str.as_ptr() as *const c_char,
                machine.machine,
                self.builder
            );
            if !error.is_null() {
                let error_msg_ptr: *mut c_char = LLVMGetErrorMessage(error);
                let c_string = CStr::from_ptr(error_msg_ptr);
                eprintln!("{}", c_string.to_str().expect("Could not read pass builder error string"));
                LLVMDisposeErrorMessage(error_msg_ptr);
                false
            } else {
                true
            }
        }
    }
}

impl Drop for PassBuilder {
    fn drop(&mut self) -> () {
        unsafe { LLVMDisposePassBuilderOptions(self.builder); }
    }
}
