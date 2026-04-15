use std::fmt;

#[derive(Debug, Clone)]
pub struct Ast {
    pub tasks: Vec<Task>,
    pub aliases: Vec<Alias>,
    pub exports: Vec<Export>,
    pub includes: Vec<Include>,
    pub dotenv: Vec<DotEnv>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub description: Option<String>,
    pub confirm: Option<String>,
    pub params: Vec<Param>,
    pub dependencies: Vec<String>,
    pub parallel_dependencies: Vec<String>,
    pub body: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Export {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Include {
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct DotEnv {
    pub path: String,
    pub line: usize,
}

impl fmt::Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.default {
            Some(def) => write!(f, "{}=\"{}\"", self.name, def),
            None => write!(f, "{}", self.name),
        }
    }
}
