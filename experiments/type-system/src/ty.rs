use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub type Bindings<'a> = HashMap<Cow<'a, str>, Type<'a>>;

// TODO: spans

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Type<'a> {
  Builtin(Builtin),
  Record(Record<'a>),
  Function(Box<Function<'a>>),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Builtin {
  Unit,
  Int,
  Bool,
  Str,
  // TODO: remove this after implementing polymorphism
  // currently it's only used to describe the type of `print`
  Any,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record<'a> {
  pub fields: Vec<Field<'a>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field<'a> {
  pub ident: Cow<'a, str>,
  pub ty: Type<'a>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Function<'a> {
  pub param: (Cow<'a, str>, Type<'a>),
  pub ret: Type<'a>,
}

impl<'a> fmt::Display for Type<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Type::Builtin(v) => write!(f, "{v}"),
      Type::Record(v) => {
        write!(f, "{{")?;
        let mut fields = v.fields.iter().peekable();
        while let Some(field) = fields.next() {
          write!(f, "{}:{}", field.ident, field.ty)?;
          if fields.peek().is_some() {
            write!(f, ", ")?;
          }
        }
        write!(f, "}}")?;
        Ok(())
      }
      Type::Function(v) => write!(f, "({}:{}) -> {}", v.param.0, v.param.1, v.ret),
    }
  }
}

impl fmt::Display for Builtin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Builtin::Unit => write!(f, "unit"),
      Builtin::Int => write!(f, "int"),
      Builtin::Bool => write!(f, "bool"),
      Builtin::Str => write!(f, "str"),
      Builtin::Any => write!(f, "any"),
    }
  }
}

impl FromStr for Builtin {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "int" => Ok(Builtin::Int),
      "bool" => Ok(Builtin::Bool),
      "str" => Ok(Builtin::Str),
      _ => Err(()),
    }
  }
}
