use std::borrow::Cow;
use std::fmt;
use std::fmt::Write;
use std::ops::Range;

use owo_colors::{AnsiColors, DynColors, OwoColorize};
use span::Span;
use thiserror::Error;

pub type DiagnosisResult<'a> = Result<Report<'a>, BuilderError>;

pub trait ToReport {
  fn to_report<'a>(&'a self, source: Source<'a>) -> DiagnosisResult<'a>;
}

#[derive(Clone)]
pub struct ReportBuilder<'a> {
  level: Level,
  source: Option<Source<'a>>,
  message: Option<Cow<'a, str>>,
  span: Option<Span>,
  label: Option<Cow<'a, str>>,
}

#[derive(Clone, Copy, Debug)]
pub enum BuilderError {
  MissingSource,
  MissingMessage,
  MissingSpan,
}

impl<'a> ReportBuilder<'a> {
  pub fn source(mut self, source: Source<'a>) -> Self {
    self.source = Some(source);
    self
  }

  pub fn message(mut self, message: Cow<'a, str>) -> Self {
    self.message = Some(message);
    self
  }

  pub fn span(mut self, span: Span) -> Self {
    self.span = Some(span);
    self
  }

  pub fn label(mut self, label: Cow<'a, str>) -> Self {
    self.label = Some(label);
    self
  }

  pub fn build(self) -> Result<Report<'a>, BuilderError> {
    use BuilderError::*;
    Ok(Report {
      level: self.level,
      source: self.source.ok_or(MissingSource)?,
      message: self.message.ok_or(MissingMessage)?,
      span: self.span.ok_or(MissingSpan)?,
      label: self.label,
    })
  }
}

#[derive(Clone, Debug)]
pub struct Source<'a> {
  name: Option<Cow<'a, str>>,
  str: Cow<'a, str>,
}

impl<'a> Source<'a> {
  pub fn string(str: Cow<'a, str>) -> Self {
    Source { name: None, str }
  }

  pub fn file(name: Cow<'a, str>, str: Cow<'a, str>) -> Self {
    Source {
      name: Some(name),
      str,
    }
  }
}

impl<'a> From<Cow<'a, str>> for Source<'a> {
  fn from(value: Cow<'a, str>) -> Self {
    Source::string(value)
  }
}

impl<'a> From<String> for Source<'a> {
  fn from(value: String) -> Self {
    Source::string(value.into())
  }
}

impl<'a> From<&'a str> for Source<'a> {
  fn from(value: &'a str) -> Self {
    Source::string(value.into())
  }
}

#[derive(Clone, Copy, Debug)]
pub enum Level {
  Info,
  Warning,
  Error,
}

pub struct Report<'a> {
  pub level: Level,
  pub source: Source<'a>,
  pub message: Cow<'a, str>,
  pub span: Span,
  pub label: Option<Cow<'a, str>>,
}

impl<'a> Report<'a> {
  pub fn info() -> ReportBuilder<'a> {
    ReportBuilder {
      level: Level::Info,
      source: None,
      message: None,
      span: None,
      label: None,
    }
  }

  pub fn warn() -> ReportBuilder<'a> {
    ReportBuilder {
      level: Level::Warning,
      source: None,
      message: None,
      span: None,
      label: None,
    }
  }

  pub fn error() -> ReportBuilder<'a> {
    ReportBuilder {
      level: Level::Error,
      source: None,
      message: None,
      span: None,
      label: None,
    }
  }

  pub fn emit<W: Write>(self, w: &mut W) -> Result<(), EmitError> {
    // examples:

    // single line span
    // error: expected semicolon
    // > foo.mu:11
    // |
    // | let y = 30_
    // |

    // multi-line span
    // error: mismatched type
    // > bar.mu:31
    // |
    // | let x: Foo = Bar {
    // |   a: 0,
    // |   b: 0,
    // | }
    // |
    // + expected "Foo", found "Bar"

    // with help
    // error: undefined variable
    // > baz.mu:12
    // |
    // | y = 10;
    // |

    // TODO: add a case for VERY LARGE multi-line snippets,
    //       where they are truncated to a max of 5 lines
    // TODO: clean this up a bit

    if self.source.str.get(Range::from(self.span)).is_none() {
      return Err(EmitError::OutOfBounds);
    }

    let snippet = Snippet::new(self.source.str.as_ref(), self.span);
    let color = match self.level {
      Level::Info => DynColors::Ansi(AnsiColors::Blue),
      Level::Warning => DynColors::Ansi(AnsiColors::Yellow),
      Level::Error => DynColors::Ansi(AnsiColors::Red),
    };

    // {level}: {message}
    match self.level {
      Level::Info => write!(w, "{}", "info".blue())?,
      Level::Warning => write!(w, "{}", "warning".yellow())?,
      Level::Error => write!(w, "{}", "error".red())?,
    }
    writeln!(w, ": {}", self.message)?;
    // > {file.name}:{line}
    writeln!(
      w,
      "{} {}:{}",
      ">".blue(),
      self.source.name.unwrap_or_else(|| "code".into()),
      snippet.line
    )?;

    writeln!(w, "{} ", "|".blue())?;
    if snippet.s[Range::from(snippet.span)].trim().is_empty() {
      // "whitespace-only" snippet, may happen in case the span is 1 character wide
      // and lands on a newline.
      let n = snippet.s[Range::from(snippet.span)].len();
      writeln!(
        w,
        "{} {}{}",
        "|".blue(),
        &snippet.s[..snippet.span.start],
        format!("{:_<width$}", "_", width = n)
          .color(color)
          .underline(),
      )?;
    } else if snippet.count > 1 {
      // multi-line snippet
      let first_lf = snippet.s[snippet.span.start..]
        .find('\n')
        .map(|i| i + snippet.span.start)
        .unwrap_or(snippet.s.len());
      let last_lf = snippet.s[snippet.span.start..snippet.span.end]
        .rfind('\n')
        .map(|i| i + snippet.span.start)
        .unwrap_or(snippet.s.len());
      writeln!(
        w,
        "{} {}{}",
        "|".blue(),
        &snippet.s[..snippet.span.start],
        snippet.s[snippet.span.start..first_lf]
          .trim()
          .color(color)
          .underline()
      )?;
      if snippet.count > 2 {
        for line in snippet.s[first_lf..last_lf].split('\n').skip(1) {
          writeln!(w, "{} {}", "|".blue(), line.color(color).underline())?;
        }
      }
      writeln!(
        w,
        "{} {}{}",
        "|".blue(),
        snippet.s[last_lf..snippet.span.end]
          .trim()
          .color(color)
          .underline(),
        &snippet.s[snippet.span.end..],
      )?;
    } else {
      // single-line snippet
      writeln!(
        w,
        "{} {}{}{}",
        "|".blue(),
        &snippet.s[..snippet.span.start],
        (&snippet.s[Range::from(snippet.span)])
          .color(color)
          .underline(),
        &snippet.s[snippet.span.end..]
      )?;
    }
    writeln!(w, "{} ", "|".blue())?;
    if let Some(label) = self.label {
      writeln!(w, "{} {}", "+".blue(), label)?;
    }

    Ok(())
  }
}

#[derive(Clone, Debug, PartialEq)]
struct Snippet<'a> {
  /// Snippet string
  pub s: Cow<'a, str>,
  /// Line number of the first line in snippet
  pub line: usize,
  /// Number of lines in this snippet
  pub count: usize,
  /// The span inside `s` which should be highlighted
  pub span: Span,
}

impl<'a> Snippet<'a> {
  pub fn new(src: &'a str, span: impl Into<Span>) -> Self {
    let span: Span = span.into();
    // the span may be multiple lines, we want to find the "full" snippet which
    // contains all the lines that the span covers.
    // for example (span is `_`):
    //   a
    //   _b
    //   cd
    //   ef_g
    //   hi
    // will yield these lines:
    //   b
    //   cd
    //   efg

    let start_line = src[..span.start].rfind('\n').unwrap_or(0);
    let end_line = src[span.end..]
      .find('\n')
      .unwrap_or_else(|| src[span.end..].len())
      + span.end;

    let s = src[start_line..end_line].trim_matches('\n');
    let line = src[..span.start].split('\n').count();
    let count = s.split('\n').count();
    let span = Span {
      start: span.start - start_line,
      end: span.end - start_line,
    };

    Self {
      s: s.into(),
      line,
      count,
      span,
    }
  }
}

#[derive(Debug, Error)]
pub enum EmitError {
  #[error("failed to format")]
  Fmt(#[from] fmt::Error),
  #[error("span does not fit within source string")]
  OutOfBounds,
}

#[cfg(test)]
mod tests {
  use super::{Report, Snippet};
  use crate::{Level, Source};

  #[test]
  fn snippet_single_line() {
    let src = "lorem ipsum dolor sit amet consectetur adipiscing elit";

    assert_eq!(
      Snippet::new(src, 6..17),
      Snippet {
        s: "lorem ipsum dolor sit amet consectetur adipiscing elit".into(),
        line: 1,
        count: 1,
        span: (6..17).into(),
      }
    );
  }

  #[test]
  fn snippet_multi_line() {
    let src = "lorem ipsum\ndolor sit amet\nconsectetur adipiscing elit";

    assert_eq!(
      Snippet::new(src, 6..17),
      Snippet {
        s: "lorem ipsum\ndolor sit amet".into(),
        line: 1,
        count: 2,
        span: (6..17).into(),
      }
    );

    let src = "lorem ipsum\ndolor sit amet\nconsectetur adipiscing elit";

    assert_eq!(
      Snippet::new(src, 17..31),
      Snippet {
        s: "dolor sit amet\nconsectetur adipiscing elit".into(),
        line: 2,
        count: 2,
        span: (6..20).into(),
      }
    );
  }

  #[test]
  fn emit_report_single_line() {
    let report = Report {
      level: Level::Error,
      source: Source {
        name: Some("test.mu".into()),
        str: "let x = 10\nlet y = 20;".into(),
      },
      message: "expected semicolon".into(),
      span: (10..11).into(),
      label: None,
    };

    let mut buf = String::new();

    report.emit(&mut buf).unwrap();

    insta::assert_snapshot!(buf);
  }

  #[test]
  fn emit_report_multi_line() {
    let report = Report {
      level: Level::Error,
      source: Source {
        name: Some("test.mu".into()),
        str: "let x: Foo = Bar {\n  a: 0,\n  b: 0,\n};".into(),
      },
      message: "mismatched type".into(),
      span: (13..36).into(),
      label: Some("expected `Foo`, found `Bar`".into()),
    };

    let mut buf = String::new();

    report.emit(&mut buf).unwrap();

    insta::assert_snapshot!(buf);
  }
}
