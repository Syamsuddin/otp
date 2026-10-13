// SPDX-License-Identifier: GPL-3.0-only
use otp_types::Type;
#[derive(Clone, Debug)]
pub enum Expr {
    Int(i64),
    Real(f64),
    Str(Vec<u8>),
    Bool(bool),
    Name(String),
    Call(String, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, String),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
}
#[derive(Clone, Debug)]
pub struct Stmt {
    pub line: usize,
    pub node: Node,
}
#[derive(Clone, Debug)]
pub enum Node {
    Block(Vec<Stmt>),
    Assign(Expr, Expr),
    Call(String, Vec<Expr>),
    Write(bool, Vec<(Expr, Option<Expr>, Option<Expr>)>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    Repeat(Vec<Stmt>, Expr),
    For(Expr, Expr, Expr, bool, Box<Stmt>),
    Case(Expr, Vec<(Vec<(i64, i64)>, Stmt)>, Vec<Stmt>),
    Break,
    Continue,
    Exit,
    Empty,
}
#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub by_ref: bool,
}
#[derive(Clone, Debug)]
pub struct Routine {
    pub name: String,
    pub params: Vec<Param>,
    pub result: Type,
    pub block: Block,
    pub line: usize,
}
#[derive(Clone, Debug)]
pub struct Block {
    pub constants: Vec<(String, Expr)>,
    pub vars: Vec<(String, Type)>,
    pub routines: Vec<Routine>,
    pub body: Stmt,
}
#[derive(Clone, Debug)]
pub struct Program {
    pub name: String,
    pub uses: Vec<String>,
    pub block: Block,
}
