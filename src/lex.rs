// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

use std::fmt;
use std::fmt::Display;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

use crate::exit_code;
use crate::options;

use exit_code::exit;
use exit_code::ExitCode;
use options::RunOptions;

#[derive(Clone,Copy,Default,Eq,PartialEq)]
pub enum TokenKind {
    #[default]
    Unknown,
    Comma,
    Comment,
    Colon,
    Eoi,
    Eol,
    Ident,
    Minus,
    Number,
    ParenL,
    ParenR,
    Plus,
    Slash,
    Star,
    With,
}

pub fn token_kind_to_string(k: TokenKind) -> String {
    match k {
        TokenKind::Comma    => "Comma",
        TokenKind::Comment  => "Comment",
        TokenKind::Colon    => "Colon",
        TokenKind::Eoi      => "Eoi",
        TokenKind::Eol      => "Eol",
        TokenKind::Ident    => "Ident",
        TokenKind::Minus    => "Minus",
        TokenKind::Number   => "Number",
        TokenKind::ParenL   => "ParenL",
        TokenKind::ParenR   => "ParenR",
        TokenKind::Plus     => "Plus",
        TokenKind::Slash    => "Slash",
        TokenKind::Star     => "Star",
        TokenKind::Unknown  => "Unknown",
        TokenKind::With     => "With",
    }.to_string()
}

#[derive(Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

impl Default for Token {
    fn default() -> Self {
        Token::new(TokenKind::Unknown, Default::default())
    }
}

impl Token {
    pub fn new(k: TokenKind, text: String) -> Self {
        Token{kind: k, text}
    }

    pub fn is(&self, k: TokenKind) -> bool {
        self.kind == k
    }

    #[allow(dead_code)]
    pub fn is_one_of(&self, ks: &[TokenKind]) -> bool {
        fn f(t: &Token, acc: bool, _ks: &[TokenKind]) -> bool {
            match _ks {
                []              => acc,
                [k]             => acc || t.is(*k),
                [k, tail @ ..]  => f(t, acc || t.is(*k), tail),
            }
        }
        f(self, false, ks)
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", token_kind_to_string(self.kind), self.text)
    }
}

pub struct Lexer<'a, T: Read> {
    buffer: BufReader<T>,
    line: String,
    line_count: usize,
    position: usize,
    options: &'a RunOptions,
}

impl <'a, T: Read> Lexer<'a, T> {
    pub fn new(readable: T, options: &'a RunOptions) -> Self {
        Lexer{
            buffer: BufReader::new(readable),
            line: String::new(),
            line_count: 0,
            position: 0,
            options,
        }
    }

    fn has_next(&mut self) -> bool {
        if self.has_next_in_line(self.position) { 
            true
        } else {
            if !self.line.is_empty() && self.position >= self.line.len() {
                self.line = Default::default();
            }
            match self.buffer.read_line(&mut self.line) {
                Ok(size) => {
                    if size > 0 {
                        if self.options.verbose {
                            eprintln!("Read {} bytes from buffer at line {}", size, self.line_count);
                        }
                        self.line_count += 1;
                        self.position = 0;
                        true
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        }
    }

    fn has_next_in_line(&self, pos: usize) -> bool {
        if self.line.is_empty() {
            false
        } else {
            pos < self.line.len()
        }
    }

    fn next_char_in_line(&self, pos: usize) -> char {
        let c_opt = self.line.get(pos..pos + 1);
        match c_opt {
            None            => {
                eprintln!("Expected char in line {} at pos {}", self.line_count - 1, pos);
                exit(ExitCode::LexerError);
            }
            Some(c_slice)   => {
                let c: char = c_slice.chars().next().unwrap();
                if !c.is_ascii() {
                    eprintln!("Only ASCII characters are supported by the lexer");
                    exit(ExitCode::LexerError);
                }
                if self.options.verbose {
                    eprintln!("Found char '{}' in line {} at pos {}", c, self.line_count - 1, pos);
                }
                c
            }
        }
    }

    fn collect_token_sequence(&self, pos: usize, pred: fn(char) -> bool) -> usize {
        let mut pos_end: usize = pos;
        let mut c: char;
        while self.has_next_in_line(pos_end) {
            c = self.next_char_in_line(pos_end);
            if !pred(c) {
                break
            }
            pos_end += 1;
        }
        pos_end
    }

    fn check_suffix(&self, pos: usize) -> () {
        if self.has_next_in_line(pos) {
            let c: char = self.next_char_in_line(pos);
            if !Self::is_whitespace(c) && !Self::is_other(c) {
                eprintln!("Found invalid suffix '{}' for number in expression", c);
                exit(ExitCode::LexerError);
            }
        }
    }

    fn next_in_line(&mut self, t: &mut Token) -> () {
        let (mut c, mut pos_start): (char, usize) = ('\0', self.position);
        while self.has_next_in_line(pos_start) {
            c = self.next_char_in_line(pos_start);
            if !Self::is_whitespace(c) { break }
            pos_start += 1;
        }
        if Self::is_whitespace(c) {
            self.form_token(t, pos_start, pos_start + 1, TokenKind::Eol);
        } else if Self::is_digit(c) {
            if c == '0' && self.has_next_in_line(pos_start + 1) {
                c = self.next_char_in_line(pos_start + 1);
                if c == 'x' {
                    let pos_end: usize = self.collect_token_sequence(pos_start + 2, Self::is_hex_digit);
                    self.check_suffix(pos_end);
                    self.form_token(t, pos_start, pos_end, TokenKind::Number);
                    return;
                }
            }
            let pos_end: usize = self.collect_token_sequence(pos_start + 1, Self::is_digit);
            self.check_suffix(pos_end);
            self.form_token(t, pos_start, pos_end, TokenKind::Number);
        } else if Self::is_letter(c) {
            let pos_end: usize = self.collect_token_sequence(pos_start + 1, Self::is_ident);
            let text = String::from(&self.line[pos_start..pos_end]);
            self.form_token(
                t,
                pos_start,
                pos_end,
                if text == "with" {TokenKind::With} else {TokenKind::Ident}
            );
        } else if Self::is_slash(c) {
            if self.has_next_in_line(pos_start + 1) {
                c = self.next_char_in_line(pos_start + 1);
                if Self::is_slash(c) {
                    // It's a comment => consume the rest of the line
                    let pos_end: usize = self.collect_token_sequence(pos_start + 2, Self::is_any);
                    self.form_token(t, pos_start, pos_end, TokenKind::Comment);
                    return;
                }
            }
            self.form_token(t, pos_start, pos_start + 1, TokenKind::Slash);
        } else {
            self.form_token(t, pos_start, pos_start + 1, match c {
                ',' => TokenKind::Comma,
                ':' => TokenKind::Colon,
                '-' => TokenKind::Minus,
                '(' => TokenKind::ParenL,
                ')' => TokenKind::ParenR,
                '+' => TokenKind::Plus,
                '/' => TokenKind::Slash,
                '*' => TokenKind::Star,
                _   => TokenKind::Unknown,
            })
        }
    }

    pub fn next(&mut self, t: &mut Token) -> () {
        let mut t_tmp: Token = Default::default();
        if self.has_next() {
            self.next_in_line(&mut t_tmp);
        } else {
            t_tmp.kind = TokenKind::Eoi;
        }
        std::mem::swap(t, &mut t_tmp);
    }

    fn form_token(&mut self, t: &mut Token, pos_start: usize, pos_end: usize, k: TokenKind) -> () {
        t.kind = k;
        t.text = String::from(
            if k == TokenKind::Eoi || k == TokenKind::Eol {
                ""
            } else {
                &self.line[pos_start..pos_end]
            }
        );
        self.position = pos_end;
    }

    fn is_any(_c: char) -> bool {
        true
    }

    fn is_slash(c: char) -> bool {
        c == '/'
    }

    fn is_other(c: char) -> bool {
        matches!(c, ',' | ':' | '-' | '(' | ')' | '+' | '/' | '*')
    }

    fn is_whitespace(c: char) -> bool {
        c == ' ' || c == '\t' || c == '\r' || c == '\n'
    }

    fn is_digit(c: char) -> bool {
        c.is_ascii_digit()
    }

    fn is_ident(c: char) -> bool {
        Self::is_digit(c) || Self::is_letter(c)
    }

    fn is_hex_digit(c: char) -> bool {
        Self::is_digit(c) || ('a'..='f').contains(&c) || ('A'..='F').contains(&c)
    }

    fn is_letter_lower(c:char) -> bool {
        c.is_ascii_lowercase()
    }

    fn is_letter_upper(c:char) -> bool {
        c.is_ascii_uppercase()
    }

    fn is_letter(c: char) -> bool {
        Self::is_letter_lower(c) || Self::is_letter_upper(c) || c == '_'
    }

    pub fn lex_input(ts: &mut Vec<Token>, lex: &mut Lexer<'a, T>, options: &RunOptions) -> () {
        let mut t: Token = Default::default();
        while !t.is(TokenKind::Eoi) {
            lex.next(&mut t);
            if t.is(TokenKind::Unknown) {
                eprintln!("Found unknown token '{}' in lexer", t.text);
                if !options.drop_token { exit(ExitCode::LexerError); }
            } else if options.verbose {
                eprintln!("Lexed token '{}'", t);
            }
            if t.is(TokenKind::Comment) || t.is(TokenKind::Eol) {
                // Drop the comments and end of lines before parsing
                continue;
            }
            ts.push(t.clone());
        }

        if options.lex_exit { exit(ExitCode::Ok); }
    }
}
