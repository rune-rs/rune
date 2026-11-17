//! The compiled instructions as rows, each with the span of source which
//! produced it, so that the editor can show which code an instruction is for.

use rune::runtime::Unit;
use rune::SourceId;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Row {
    Function {
        signature: String,
        hash: String,
    },
    Label {
        name: String,
    },
    Inst {
        ip: String,
        op: String,
        args: Vec<Arg>,
        comment: Option<String>,
        /// A byte range into the compiled source.
        span: Option<(usize, usize)>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Arg {
    pub key: String,
    pub value: String,
}

/// Rows from the text `Unit::emit_instructions` writes without source, which
/// is one line per function header, label or instruction.
pub fn rows(unit: &Unit, source_id: SourceId, text: &str) -> Vec<Row> {
    let debug = unit.debug_info();
    let mut rows = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("  ") {
            let Some((ip, rest)) = rest.split_once(" = ") else {
                continue;
            };

            let (body, comment) = match rest.split_once(" // ") {
                Some((body, comment)) => (body, Some(comment.to_owned())),
                None => (rest, None),
            };

            let (op, args) = body.split_once(' ').unwrap_or((body, ""));

            let span = ip
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|ip| debug?.instruction_at(ip))
                .filter(|inst| inst.source_id == source_id)
                .map(|inst| (inst.span.start.into_usize(), inst.span.end.into_usize()));

            rows.push(Row::Inst {
                ip: ip.trim().to_owned(),
                op: op.to_owned(),
                args: split_args(args),
                comment,
                span,
            });
        } else if let Some(rest) = line.strip_prefix("fn ") {
            let rest = rest.trim_end_matches(':');

            let (signature, hash) = match rest.rsplit_once(" (") {
                Some((signature, hash)) => (signature, hash.trim_end_matches(')')),
                None => (rest, ""),
            };

            rows.push(Row::Function {
                signature: signature.to_owned(),
                hash: hash.to_owned(),
            });
        } else {
            rows.push(Row::Label {
                name: line.trim_end_matches(':').to_owned(),
            });
        }
    }

    rows
}

/// `addr=2, args=1, out=keep(3)`, where a value may itself contain commas
/// inside brackets.
fn split_args(args: &str) -> Vec<Arg> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;

    let mut push = |part: &str| {
        let part = part.trim();

        if part.is_empty() {
            return;
        }

        let (key, value) = part.split_once('=').unwrap_or(("", part));

        out.push(Arg {
            key: key.to_owned(),
            value: value.to_owned(),
        });
    };

    for (at, c) in args.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                push(&args[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }

    push(&args[start..]);
    out
}
