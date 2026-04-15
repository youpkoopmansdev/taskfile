#![allow(dead_code)]

use std::fmt;

#[derive(Debug, Clone)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl Span {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    pub fn point(line: u32, col: u32) -> Self {
        Self {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub tasks: Vec<Task>,
    pub aliases: Vec<Alias>,
    pub exports: Vec<Export>,
    pub includes: Vec<Include>,
    pub dotenv: Vec<DotEnv>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub name_span: Span,
    pub description: Option<String>,
    pub confirm: Option<String>,
    pub params: Vec<Param>,
    pub dependencies: Vec<Dependency>,
    pub parallel_dependencies: Vec<Dependency>,
    pub body: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default: Option<String>,
    pub span: Span,
}

impl fmt::Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.default {
            Some(def) => write!(f, "{}=\"{}\"", self.name, def),
            None => write!(f, "{}", self.name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Export {
    pub key: String,
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Include {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct DotEnv {
    pub path: String,
    pub span: Span,
}
