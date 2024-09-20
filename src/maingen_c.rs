// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

/// Generate LLVM IR for a main function, using the C standard library, from `main.c.template`.
/// Enable this using `--c-main`/`-C`.

extern crate llvm_sys as llvm;
use llvm::prelude::LLVMTypeRef;

use std::env;
use std::path::Path;
use std::str;

use crate::command;
use crate::module;
use crate::exit_code;

use command::Command;
use exit_code::exit;
use exit_code::ExitCode;
use module::FunctionSignature;
use module::ModuleBundle;

pub struct MainGenC {}

const INPUT_NUM_ARGS        : &str = "@@NUM_ARGS";
const INPUT_USAGE_ARGS      : &str = "@@USAGE_ARGS";
const INPUT_PARAM_TYPES_LIST: &str = "@@PARAM_TYPES_LIST";
const INPUT_PARAM_DECLS_LIST: &str = "@@PARAM_DECLS_LIST";
const INPUT_PARAMS_LIST     : &str = "@@PARAMS_LIST";
const MAIN_C_TEMPLATE       : &str = include_str!("main.c.template");

const CLANG_AGS: [&str; 9] = [
    "-x", "c", "-", "-Wall", "-Werror", "-std=c99", "-fPIC", "-c", "-o"
];

impl MainGenC {
    /// Returns a path (option) to the object file generated from `main.c.tempalte`.
    /// This object file will be linked to the object file generated from the LLVM during the
    /// IRGen stage.
    pub fn gen(callee_sig: &FunctionSignature, stem: &str, verbose: bool) -> Option<String> {
        let body = Self::substitute_all_inputs_in_body(MAIN_C_TEMPLATE, callee_sig, verbose);
        let mut clang_args: Vec<&str> = Vec::from(CLANG_AGS);
        let dir = env::temp_dir();
        let main_obj_path = Self::get_main_obj_path(&dir, stem);
        if verbose {
            eprintln!("Path to main object file: {}", main_obj_path);
        }
        clang_args.push(main_obj_path.as_str());
        let clang_result = Command::run_with_input("clang", &clang_args, &body);
        if clang_result.stdout.is_some() {
            println!("{}", clang_result.stdout.unwrap());
        }
        if clang_result.success {
            Some(main_obj_path)
        } else {
            None
        }
    }

    fn get_main_obj_path(dir: &Path, stem: &str) -> String {
        match dir.join(format!("main.{}.o", stem)).as_path().to_str() {
            Some(path)  => path,
            None        => {
                eprintln!("Failed to create path for main.o");
                exit(ExitCode::MainGenCError);
            },
        }.to_string()
    }

    fn substitute_all_inputs_in_body(body: &str, callee_sig: &FunctionSignature, verbose: bool) -> String {
        let num_args = format!("{}", callee_sig.params.len());
        let usage_string = Self::get_usage_args_string(&callee_sig.params);
        let param_decls = Self::collect_callee_param_decls_string(&callee_sig.params, 1);
        let param_types_list = Self::get_callee_param_types_list_string(&callee_sig.params);
        let params_list = Self::get_callee_params_list_string(&callee_sig.params);
        let body_with_num_args = Self::substitute_param_in_string(body, INPUT_NUM_ARGS, &num_args);
        let body_with_usage_args = Self::substitute_param_in_string(
            &body_with_num_args, INPUT_USAGE_ARGS, &usage_string
        );
        let body_with_param_decls = Self::substitute_param_in_string(
            &body_with_usage_args, INPUT_PARAM_DECLS_LIST, &param_decls
        );
        let body_with_param_types_list = Self::substitute_param_in_string(
            &body_with_param_decls, INPUT_PARAM_TYPES_LIST, &param_types_list
        );
        let body_with_params_list = Self::substitute_param_in_string(
            &body_with_param_types_list, INPUT_PARAMS_LIST, &params_list
        );
        if verbose {
            eprintln!("Body of 'main.c' after input substitution:\n{}", body_with_params_list);
        }
        body_with_params_list
    }

    fn get_usage_args_string(params: &[LLVMTypeRef]) -> String {
        let mut args_string = String::new();
        for i in 0..params.len() {
            let sep = if i == 0 { "" } else { ", " };
            args_string = args_string + sep + format!("<arg{}>", i).as_str();
        }
        args_string
    }

    fn collect_callee_param_decls_string(params: &[LLVMTypeRef], indent_depth: usize) -> String {
        let mut decls: Vec<String> = Vec::new();
        for (i, t) in params.iter().enumerate() {
            decls.push(Self::get_callee_param_decl_string(i, *t));
        }
        let mut join_str = String::from("\n");
        for _ in 0..indent_depth {
            join_str += "    ";
        }
        decls.join(join_str.as_str())
    }

    /// Generates an assignment to a constant integer from a call to atoll
    fn get_callee_param_decl_string(idx: usize, t: LLVMTypeRef) -> String {
        let t_str = format!("t_{}", ModuleBundle::type_name_from(t));
        format!("const {} p{} = ({})atoll(argv[BASE + {}]);", t_str, idx, t_str, idx)
    }

    fn get_callee_param_types_list_string(params: &[LLVMTypeRef]) -> String {
        let mut params_string = String::new();
        for (i, t) in params.iter().enumerate() {
            let sep = if i == 0 { "" } else { ", " };
            let t_name = format!("t_{}", ModuleBundle::type_name_from(*t));
            params_string = params_string + sep + &t_name;
        }
        params_string
    }

    fn get_callee_params_list_string(params: &[LLVMTypeRef]) -> String {
        let n = params.len();
        let mut params_string = String::new();
        for i in 0..n {
            let sep = if i == 0 { "" } else { ", " };
            params_string = params_string + sep + format!("p{}", i).as_str();
        }
        params_string
    }

    fn substitute_param_in_string(body: &str, old: &str, new: &str) -> String {
        str::replace(body, old, new).to_string()
    }
}
