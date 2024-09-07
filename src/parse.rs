// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

use std::str::FromStr;

use crate::ast;
use crate::exit_code;
use crate::lex;
use crate::options;

use ast::Ast;
use ast::Expr;
use ast::Operator;
use exit_code::exit;
use exit_code::ExitCode;
use lex::token_kind_to_string;
use lex::Token;
use lex::TokenKind;
use options::RunOptions;

#[derive(Clone)]
pub struct ParserIter {
    token: Token,
    vars: Vec<String>,
    position: usize,
    end: usize,
}

impl ParserIter {
    pub fn new(end: usize) -> Self {
        ParserIter{
            token: Default::default(),
            vars: Vec::new(),
            position: 0,
            end: end,
        }
    }

    pub fn has_next(&self) -> bool {
        self.position < self.end
    }
}

pub struct Parser<'a> {
    tokens: &'a Vec<Token>,
    options: &'a RunOptions,
}

impl <'a> Parser<'a> {
    pub fn new(tokens: &'a Vec<Token>, options: &'a RunOptions) -> Self {
        if tokens.len() < 1 {
            eprintln!("Found empty program while parsing");
            exit(ExitCode::ParserError);
        }
        Parser{
            tokens: tokens,
            options: options,
        }
    }

    pub fn iter(&self) -> ParserIter {
        ParserIter::new(self.tokens.len())
    }

    fn consume(&self, iter: &mut ParserIter, k: TokenKind, add_var: bool) -> bool {
        let t: &Token = self.get_token(iter);
        if t.is(k) {
            if self.options.verbose {
                eprintln!("Consumed expected token '{}' at position '{}'", token_kind_to_string(k), iter.position);
            }
            iter.token = t.clone();
            iter.position += 1;
            if add_var && t.is(TokenKind::Ident) {
                iter.vars.push(t.text.clone()); 
            }
            true
        } else {
            false
        }
    }

    fn consume_one_of(&self, iter: &mut ParserIter, ks: &[TokenKind], add_var: bool) -> bool {
        for k in ks {
            if self.consume(iter, *k, add_var) { return true }
        }
        false
    }
    
    fn expect(&self, iter: &mut ParserIter, k: TokenKind, add_var: bool) {
        if self.consume(iter, k, add_var) {
            return
        } else {
            eprintln!("Expected '{}' token at position {}", token_kind_to_string(k), iter.position);
            exit(ExitCode::ParserError);
        }
    }

    fn get_prev_token(&'a self, iter: &'a mut ParserIter) -> &Token {
        &iter.token
    }

    fn get_token(&self, iter: &mut ParserIter) -> &Token {
        if iter.has_next() {
            &self.tokens.get(iter.position).unwrap()
        } else {
            eprintln!("Token out of bounds at {}", iter.position);
            exit(ExitCode::ParserError);
        }
    }

    fn parse_calc(&self, iter: &mut ParserIter) -> Box<&mut dyn Ast> {
        let mut expr: Box<Expr>;
        if self.consume(iter, TokenKind::With, false) {
            self.expect(iter, TokenKind::Colon, false);
            self.expect(iter, TokenKind::Ident, true);
            while self.consume(iter, TokenKind::Comma, false) {
                self.expect(iter, TokenKind::Ident, true);
            }
            self.expect(iter, TokenKind::Colon, false);
            expr = self.parse_expr(iter);
            expr = Box::new(Expr::new_withdecl(iter.vars.clone(), Box::leak(expr)));
        } else {
            expr = self.parse_expr(iter);
        }
        Box::new(Box::leak(expr) as &mut dyn Ast)
    }

    fn parse_expr(&self, iter: &mut ParserIter) -> Box<Expr> {
        let mut e_left: Box<Expr> = self.parse_term(iter);
        while self.consume_one_of(iter, &[TokenKind::Plus, TokenKind::Minus], false) {
            let e_op: Operator = match self.get_prev_token(iter).kind {
                TokenKind::Plus     => Operator::Add,
                TokenKind::Minus    => Operator::Sub,
                _                   => {
                    eprintln!("Unxpected token");
                    exit(ExitCode::ParserError);
                }
            };
            let e_right: Box<Expr> = self.parse_term(iter);
            e_left = Box::new(Expr::new_binop(e_op, Box::leak(e_left), Box::leak(e_right)));
        }
        e_left
    }

    fn parse_term(&self, iter: &mut ParserIter) -> Box<Expr> {
        let mut e_left: Box<Expr> = self.parse_factor(iter);
        while self.consume_one_of(iter, &[TokenKind::Star, TokenKind::Slash], false) {
            let e_op: Operator = match self.get_prev_token(iter).kind {
                TokenKind::Star     => Operator::Mul, 
                TokenKind::Slash    => Operator::Div,
                _                   => {
                    eprintln!("Unxpected token");
                    exit(ExitCode::ParserError);
                }
            };
            let e_right: Box<Expr> = self.parse_factor(iter);
            e_left = Box::new(Expr::new_binop(e_op, Box::leak(e_left), Box::leak(e_right)));
        }
        e_left
    }

    fn is_hex_number(text: &String) -> bool {
        text.len() >= 2 && "0x" == &text[0..2]
    }

    fn str_to_number(text: &String) -> i64 {
        let (result, msg) = if Self::is_hex_number(text) {
            (i64::from_str_radix(&text[2..], 16), "Failed to convert hexadecimal string")
        } else {
            (i64::from_str(text.as_str()), "Failed to convert decimal string")
        };
        match result {
            Ok(n)   => n,
            Err(e)  => {
                eprintln!("Number '{}' failed parse: {}\n{}", text, e, msg);
                exit(ExitCode::ParserError);
            },
        }
    }

    fn parse_factor(&self, iter: &mut ParserIter) -> Box<Expr> {
        if self.consume(iter, TokenKind::Minus, false) {
			// NOTE: Implement unary minus as for identifiers as BinaryOp(Sub,0,..) and numbers as -<num>
			if self.consume(iter, TokenKind::Number, false) {
				let text = format!("-{}", self.get_prev_token(iter).text.clone());
				let n = Self::str_to_number(&text);
				Box::new(Expr::new_number(n))
			} else if self.consume(iter, TokenKind::Ident, false) {
				let zero = Box::new(Expr::new_number(0));
				let ident = Box::new(Expr::new_ident(self.get_prev_token(iter).text.clone()));
				Box::new(Expr::new_binop(Operator::Sub, Box::leak(zero), Box::leak(ident)))
			} else if self.consume(iter, TokenKind::ParenL, false) {
				let zero = Box::new(Expr::new_number(0));
				let expr = self.parse_expr(iter);
				self.expect(iter, TokenKind::ParenR, false);
				Box::new(Expr::new_binop(Operator::Sub, Box::leak(zero), Box::leak(expr)))
			} else {
				eprintln!("Unexpected token after Token:Minus");
				exit(ExitCode::ParserError);
			}
        } else if self.consume(iter, TokenKind::Number, false) {
            let n = Self::str_to_number(&self.get_prev_token(iter).text.clone());
            Box::new(Expr::new_number(n))
        } else if self.consume(iter, TokenKind::Ident, false) {
            Box::new(Expr::new_ident(self.get_prev_token(iter).text.clone()))
        } else if self.consume(iter, TokenKind::ParenL, false) {
            let expr = self.parse_expr(iter);
            self.expect(iter, TokenKind::ParenR, false);
            expr
        } else {
            eprintln!("Unxpected token");
            exit(ExitCode::ParserError);
        }
    }

    pub fn parse_input(ret: &mut Box<&'a mut dyn Ast>, parser: &'a mut Parser<'a>, options: &RunOptions) {
        let mut iter = parser.iter();
        *ret = parser.parse_calc(&mut iter);
        if options.print_ast { eprintln!("AST: {}", ret.to_string()); }
        if options.parse_exit { exit(ExitCode::Ok); }
    }
}
