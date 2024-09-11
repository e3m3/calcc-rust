// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

/// Manually generate LLVM IR for a main function, using the C standard library, which calls the calcc
/// program function generated for the input program.
/// This approach is used to avoid interfacing with a stub program written in C, directly calling
/// the dependent library functions.
/// Omit this code generation by passing '--nomain' to calcc.

extern crate llvm_sys as llvm;

use llvm::core::*;
use llvm::prelude::LLVMBasicBlockRef;
use llvm::prelude::LLVMBool;
use llvm::prelude::LLVMTypeRef;
use llvm::prelude::LLVMValueRef;
use llvm::LLVMIntPredicate;

use std::ffi::c_char;
use std::ffi::c_uint;

use crate::module;

use module::FunctionSignature;
use module::ModuleBundle;

static NAME_ATOLL       : &str = "atoll";
static NAME_CALCC_MAIN  : &str = "calcc_main";
static NAME_FPRINTF     : &str = "fprintf";
static NAME_PRINTF      : &str = "printf";
static NAME_STDERR      : &str = "stderr";

static NAME_ARG_ERR     : &str = ".str.argerr";
static STRING_ARG_ERR   : &str = "Invalid number of args to main. Expected %d args\n\0";
static NAME_RESULT_STR  : &str = ".str.result";
static STRING_RESULT_STR: &str = "calcc_main result: %lld\n\0";
static NAME_USAGE       : &str = ".str.usage";
static STRING_USAGE     : &str = "<exe> [<arg0>, <arg1>, ...]\n\0";

static NAME_RETVAL      : &str = "vRet";
static NAME_ARGC        : &str = "vArgc";
static NAME_ARGV        : &str = "vArgv";

pub struct MainGen<'a, 'b> {
    bundle: &'a mut ModuleBundle<'b>,
}

impl <'a, 'b> MainGen<'a, 'b> {
    pub fn new(bundle: &'a mut ModuleBundle<'b>) -> Self {
        MainGen{bundle}
    }

    pub fn gen(bundle: &'a mut ModuleBundle<'b>, callee_sig: &'a FunctionSignature) -> bool {
        let mut maingen: Self = Self::new(bundle);
        let _printf = maingen.declare_atoll();
        let _fprintf = maingen.declare_fprintf();
        let _printf = maingen.declare_printf();
        let _stderr = maingen.declare_stderr();
        let _calcc_main = maingen.declare_calcc_main(callee_sig);
        let bb_entry = maingen.make_entry_block();
        let bb_err = maingen.make_err_block();
        let bb_body = maingen.make_body_block();
        let bb_ret = maingen.make_ret_block();
        maingen.declare_global_strings(); // NOTE: Needs to be called after first use of builder
        let callee_values = maingen.gen_entry_stack(bb_entry, callee_sig);
        maingen.gen_entry_branch(bb_entry, bb_err, bb_body, callee_sig.params.len());
        maingen.gen_err_block(bb_err, bb_ret, callee_sig.params.len());
        maingen.gen_body(bb_body, bb_ret, callee_sig, &callee_values);
        maingen.gen_ret(bb_ret);
        true
    }

    fn make_entry_block(&mut self) -> LLVMBasicBlockRef {
        let mut param_types: Vec<LLVMTypeRef> = vec![self.bundle.t_i32, self.bundle.t_opaque];
        let f_name = ModuleBundle::value_name(self.bundle.name.as_str());
        unsafe {
            let t_ret = self.bundle.t_i32;
            let t_f = LLVMFunctionType(t_ret, param_types.as_mut_ptr(), param_types.len() as u32, false as LLVMBool);
            let f = LLVMAddFunction(self.bundle.module, f_name.as_ptr() as *const c_char, t_f);
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

    fn gen_entry_branch(
        &mut self,
        bb_entry: LLVMBasicBlockRef,
        bb_err: LLVMBasicBlockRef,
        bb_body: LLVMBasicBlockRef,
        callee_arg_len: usize,
    ) -> () {
        unsafe { LLVMPositionBuilderAtEnd(self.bundle.builder, bb_entry); };
        let name_ret = ModuleBundle::value_name(NAME_RETVAL);
        let name_argc = ModuleBundle::value_name(NAME_ARGC);
        let name_argv = ModuleBundle::value_name(NAME_ARGV);
        let value_ret = self.bundle.get_value(&name_ret);
        let value_argc = self.bundle.get_value(&name_argc);
        let value_argv = self.bundle.get_value(&name_argv);
        let init_value_zero = self.bundle.get_constint(self.bundle.t_i32, 0);
        let cmp_value_args = self.bundle.get_constint(self.bundle.t_i32, (callee_arg_len + 1) as i64);
        let name_argc_tmp = self.bundle.scope.next_value_name();
        let name_icmp = self.bundle.scope.next_value_name();
        let f = self.bundle.f.unwrap();
        unsafe {
            let init_value_argc = LLVMGetParam(f, 0 as c_uint);
            let init_value_argv = LLVMGetParam(f, 1 as c_uint);
            let _ = LLVMBuildStore(self.bundle.builder, init_value_zero, value_ret);
            let _ = LLVMBuildStore(self.bundle.builder, init_value_argc, value_argc);
            let _ = LLVMBuildStore(self.bundle.builder, init_value_argv, value_argv);
            let value_argc_tmp = LLVMBuildLoad2(
                self.bundle.builder,
                self.bundle.t_i32,
                value_argc,
                name_argc_tmp.as_ptr() as *const c_char
            );
            let value_icmp = LLVMBuildICmp(
                self.bundle.builder,
                LLVMIntPredicate::LLVMIntNE,
                value_argc_tmp,
                cmp_value_args,
                name_icmp.as_ptr() as *const c_char
            );
            let _ = LLVMBuildCondBr(self.bundle.builder, value_icmp, bb_err, bb_body);
        };
    }

    fn make_err_block(&mut self) -> LLVMBasicBlockRef {
        let f = self.bundle.f.unwrap();
        unsafe {
            let bb = LLVMAppendBasicBlockInContext(
                self.bundle.context,
                f,
                ModuleBundle::value_name("print_err").as_ptr() as *const c_char,
            );
            LLVMPositionBuilderAtEnd(self.bundle.builder, bb);
            bb
        }
    }

    fn make_body_block(&mut self) -> LLVMBasicBlockRef {
        let f = self.bundle.f.unwrap();
        unsafe {
            let bb = LLVMAppendBasicBlockInContext(
                self.bundle.context,
                f,
                ModuleBundle::value_name("body").as_ptr() as *const c_char,
            );
            LLVMPositionBuilderAtEnd(self.bundle.builder, bb);
            bb
        }
    }

    fn gen_body(
        &mut self,
        bb_body: LLVMBasicBlockRef,
        bb_ret: LLVMBasicBlockRef,
        callee_sig: &'a FunctionSignature,
        callee_values: &[LLVMValueRef]
    ) -> () {
        unsafe { LLVMPositionBuilderAtEnd(self.bundle.builder, bb_body); }
        let name_retval = ModuleBundle::value_name(NAME_RETVAL);
        let name_printf = ModuleBundle::value_name(NAME_PRINTF);
        let value_retval = self.bundle.get_value(&name_retval);
        let value_printf = self.bundle.get_value(&name_printf);
        let value_errcode = self.bundle.get_constint(self.bundle.t_i32, 0);
        for i in 1..callee_values.len() {
            let value_store = callee_values.get(i).unwrap();
            self.argv_to_atoll(*value_store, i as i32);
        }
        let value_result = self.gen_call_calcc_main(callee_sig, callee_values);
        let name_load_tmp = self.bundle.scope.next_value_name();
        let value_load_tmp = unsafe {
            LLVMBuildLoad2(
                self.bundle.builder,
                callee_sig.t_ret,
                value_result,
                name_load_tmp.as_ptr() as *const c_char
            )
        };
        let name_result_str = ModuleBundle::value_name(NAME_RESULT_STR);
        let name_printf_tmp = self.bundle.scope.next_value_name();
        let value_result_str = self.bundle.get_value(&name_result_str);
        let mut param_values: Vec<LLVMValueRef> = vec![value_result_str, value_load_tmp];
        let mut param_types: Vec<LLVMTypeRef> = vec![self.bundle.t_opaque, callee_sig.t_ret];
        let n = param_types.len();
        unsafe {
            let t_f = LLVMFunctionType(self.bundle.t_i32, param_types.as_mut_ptr(), n as u32, true as LLVMBool);
            let _ = LLVMBuildCall2(
                self.bundle.builder,
                t_f,
                value_printf,
                param_values.as_mut_ptr(),
                n as c_uint,
                name_printf_tmp.as_ptr() as *const c_char
            );
            let _ = LLVMBuildStore(self.bundle.builder, value_errcode, value_retval);
            let _ = LLVMBuildBr(self.bundle.builder, bb_ret);
        }
    }

    fn gen_entry_stack(&mut self, bb_entry: LLVMBasicBlockRef, f_sig: &'a FunctionSignature) -> Vec<LLVMValueRef> {
        unsafe { LLVMPositionBuilderAtEnd(self.bundle.builder, bb_entry); }
        let _value_ret = self.bundle.gen_alloca(NAME_RETVAL, self.bundle.t_i32);
        let _value_argc = self.bundle.gen_alloca(NAME_ARGC, self.bundle.t_i32);
        let _value_argv = self.bundle.gen_alloca(NAME_ARGV, self.bundle.t_opaque);
        let name_result = self.bundle.scope.next_value_name();
        let value_result = self.bundle.gen_alloca(&name_result, f_sig.t_ret);
        let mut v: Vec<LLVMValueRef> = vec![value_result];
        for t in f_sig.params.iter() {
            let value_name = self.bundle.scope.next_value_name();
            let value_param = self.bundle.gen_alloca(&value_name, *t);
            v.push(value_param);
        }
        v
    }

    fn make_ret_block(&mut self) -> LLVMBasicBlockRef {
        let f = self.bundle.f.unwrap();
        let name_block = ModuleBundle::value_name("ret_label");
        unsafe {
            let bb = LLVMAppendBasicBlockInContext(self.bundle.context, f, name_block.as_ptr() as *const c_char);
            LLVMPositionBuilderAtEnd(self.bundle.builder, bb);
            bb
        }
    }

    fn gen_ret(&mut self, bb_ret: LLVMBasicBlockRef) -> () {
        unsafe { LLVMPositionBuilderAtEnd(self.bundle.builder, bb_ret); }
        let ret_value_name = ModuleBundle::value_name(NAME_RETVAL);
        let alloca_value: LLVMValueRef = self.bundle.get_value(&ret_value_name);
        let value_name = self.bundle.scope.next_value_name();
        unsafe {
            let ret_value = LLVMBuildLoad2(
                self.bundle.builder,
                self.bundle.t_i32,
                alloca_value,
                value_name.as_ptr() as *const c_char
            );
            LLVMBuildRet(self.bundle.builder, ret_value);
        }
    }

    fn gen_err_block(&mut self, bb_err: LLVMBasicBlockRef, bb_ret: LLVMBasicBlockRef, callee_arg_len: usize) -> () {
        unsafe { LLVMPositionBuilderAtEnd(self.bundle.builder, bb_err) };
        let name_retval = ModuleBundle::value_name(NAME_RETVAL);
        let name_stderr = ModuleBundle::value_name(NAME_STDERR);
        let name_fprintf = ModuleBundle::value_name(NAME_FPRINTF);
        let name_arg_err = ModuleBundle::value_name(NAME_ARG_ERR);
        let name_usage = ModuleBundle::value_name(NAME_USAGE);
        let value_retval = self.bundle.get_value(&name_retval);
        let value_stderr = self.bundle.get_value(&name_stderr);
        let value_fprintf = self.bundle.get_value(&name_fprintf);
        let value_arg_err = self.bundle.get_value(&name_arg_err);
        let value_usage = self.bundle.get_value(&name_usage);
        let name_stderr_tmp = self.bundle.scope.next_value_name();
        let name_call_tmp1 = self.bundle.scope.next_value_name();
        let name_call_tmp2 = self.bundle.scope.next_value_name();
        let value_num_argc = self.bundle.get_constint(self.bundle.t_i32, callee_arg_len as i64);
        let value_errcode = self.bundle.get_constint(self.bundle.t_i32, 1);
        let value_stderr_tmp = unsafe { LLVMBuildLoad2(
            self.bundle.builder,
            self.bundle.t_opaque,
            value_stderr,
            name_stderr_tmp.as_ptr() as *const c_char
        )};
        let mut params_fprintf1: Vec<LLVMValueRef> = vec![value_stderr_tmp, value_arg_err, value_num_argc];
        let mut params_fprintf2: Vec<LLVMValueRef> = vec![value_stderr_tmp, value_usage];
        let mut param_types: Vec<LLVMTypeRef> = vec![self.bundle.t_opaque, self.bundle.t_opaque];
        unsafe {
            let t_f = LLVMFunctionType(
                self.bundle.t_i32,
                param_types.as_mut_ptr(),
                param_types.len() as u32,
                true as LLVMBool
            );
            let _ = LLVMBuildCall2(
                self.bundle.builder,
                t_f,
                value_fprintf,
                params_fprintf1.as_mut_ptr(),
                params_fprintf1.len() as c_uint,
                name_call_tmp1.as_ptr() as *const c_char
            );
            let _ = LLVMBuildCall2(
                self.bundle.builder,
                t_f,
                value_fprintf,
                params_fprintf2.as_mut_ptr(),
                params_fprintf2.len() as c_uint,
                name_call_tmp2.as_ptr() as *const c_char
            );
            let _ = LLVMBuildStore(self.bundle.builder, value_errcode, value_retval);
            let _ = LLVMBuildBr(self.bundle.builder, bb_ret);
        };
    }

    fn argv_to_atoll(&mut self, value_store: LLVMValueRef, idx: i32) -> () {
        let name_argv = ModuleBundle::value_name(NAME_ARGV);
        let name_atoll = ModuleBundle::value_name(NAME_ATOLL);
        let value_atoll = self.bundle.get_value(&name_atoll);
        let value_argv = self.bundle.get_value(&name_argv);
        let value_idx = self.bundle.get_constint(self.bundle.t_i32, idx as i64);
        let name_load_tmp1 = self.bundle.scope.next_value_name();
        let name_gep_tmp = self.bundle.scope.next_value_name();
        let name_load_tmp2 = self.bundle.scope.next_value_name();
        let name_call_tmp = self.bundle.scope.next_value_name();
        let mut indices: Vec<LLVMValueRef> = vec![value_idx];
        let value_load_tmp2 = unsafe {
            let value_load_tmp1 = LLVMBuildLoad2(
                self.bundle.builder,
                self.bundle.t_opaque,
                value_argv,
                name_load_tmp1.as_ptr() as *const c_char
            );
            let value_gep_tmp = LLVMBuildGEP2(
                self.bundle.builder,
                self.bundle.t_opaque,
                value_load_tmp1,
                indices.as_mut_ptr(),
                indices.len() as c_uint,
                name_gep_tmp.as_ptr() as *const c_char
            );
            LLVMBuildLoad2(
                self.bundle.builder,
                self.bundle.t_opaque,
                value_gep_tmp,
                name_load_tmp2.as_ptr() as *const c_char
            )
        };
        let mut params_atoll: Vec<LLVMValueRef> = vec![value_load_tmp2];
        let mut param_types: Vec<LLVMTypeRef> = vec![self.bundle.t_opaque];
        unsafe {
            let t_f = LLVMFunctionType(
                self.bundle.t_i64,
                param_types.as_mut_ptr(),
                param_types.len() as u32,
                false as LLVMBool
            );
            let value_call_tmp = LLVMBuildCall2(
                self.bundle.builder,
                t_f,
                value_atoll,
                params_atoll.as_mut_ptr(),
                params_atoll.len() as c_uint,
                name_call_tmp.as_ptr() as *const c_char
            );
            let _ = LLVMBuildStore(
                self.bundle.builder,
                value_call_tmp,
                value_store
            );
        }
    }

    fn gen_call_calcc_main(
        &mut self,
        callee_sig: &'a FunctionSignature,
        callee_values: &[LLVMValueRef]
    ) -> LLVMValueRef {
        let name_calcc = ModuleBundle::value_name(NAME_CALCC_MAIN);
        let value_calcc = self.bundle.get_value(&name_calcc);
        let value_result = callee_values.first().unwrap();
        let mut args: Vec<LLVMValueRef> = Vec::new();
        for i in 1..callee_values.len() {
            let name = self.bundle.scope.next_value_name();
            let t_arg = callee_sig.params.get(i - 1).expect("The first item in callee_values should be the return value");
            let value_arg = callee_values.get(i).unwrap();
            let value_load = unsafe {
                LLVMBuildLoad2(
                    self.bundle.builder,
                    *t_arg,
                    *value_arg,
                    name.as_ptr() as *const c_char
                )
            };
            args.push(value_load);
        }
        let name_call_tmp = self.bundle.scope.next_value_name();
        unsafe {
            let t_f = LLVMFunctionType(
                callee_sig.t_ret,
                callee_sig.params.clone().as_mut_ptr(),
                callee_sig.params.len() as u32,
                false as LLVMBool
            );
            let value_call_tmp = LLVMBuildCall2(
                self.bundle.builder,
                t_f,
                value_calcc,
                args.as_mut_ptr(),
                args.len() as c_uint,
                name_call_tmp.as_ptr() as *const c_char
            );
            let _ = LLVMBuildStore(
                self.bundle.builder,
                value_call_tmp,
                *value_result
            );
        };
        *value_result
    }

    fn set_align(&mut self, value: LLVMValueRef, align: u8) -> () {
        unsafe { LLVMSetAlignment(value, align as c_uint); }
    }

    fn declare_global_strings(&mut self) -> () {
        let _value_argerr: LLVMValueRef = self.bundle.declare_global_string(NAME_ARG_ERR, STRING_ARG_ERR);
        let _value_result_str: LLVMValueRef = self.bundle.declare_global_string(NAME_RESULT_STR, STRING_RESULT_STR);
        let _value_usage: LLVMValueRef = self.bundle.declare_global_string(NAME_USAGE, STRING_USAGE);
    }

    fn declare_stderr(&mut self) -> LLVMValueRef {
        let name: String = ModuleBundle::value_name(NAME_STDERR);
        let value: LLVMValueRef = unsafe {
            let value: LLVMValueRef = LLVMAddGlobal(
                self.bundle.module,
                self.bundle.t_opaque,
                name.as_ptr() as *const c_char
            );
            value
        };
        self.set_align(value, 8);
        self.bundle.insert_value(&name, value);
        value
    }

    fn declare_atoll(&mut self) -> LLVMValueRef {
        let name: String = ModuleBundle::value_name(NAME_ATOLL);
        let mut params: Vec<LLVMTypeRef> = vec![self.bundle.t_opaque];
        self.bundle.emit_declaration(&name, self.bundle.t_i64, &mut params, false)
    }

    fn declare_calcc_main(&mut self, f_sig: &'a FunctionSignature) -> LLVMValueRef {
        let name: String = ModuleBundle::value_name(NAME_CALCC_MAIN);
        let mut params = f_sig.params.clone();
        self.bundle.emit_declaration(&name, f_sig.t_ret, &mut params, false)
    }

    fn declare_printf(&mut self) -> LLVMValueRef {
        let name: String = ModuleBundle::value_name(NAME_PRINTF);
        let mut params: Vec<LLVMTypeRef> = vec![self.bundle.t_opaque];
        self.bundle.emit_declaration(&name, self.bundle.t_i32, &mut params, true)
    }

    fn declare_fprintf(&mut self) -> LLVMValueRef {
        let name: String = ModuleBundle::value_name(NAME_FPRINTF);
        let mut params: Vec<LLVMTypeRef> = vec![self.bundle.t_opaque, self.bundle.t_opaque];
        self.bundle.emit_declaration(&name, self.bundle.t_i32, &mut params, true)
    }
}
