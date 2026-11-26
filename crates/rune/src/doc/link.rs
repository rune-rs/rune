//! Resolving intra-doc links.
//!
//! A link like ``[`Option::map`]`` in the documentation of an item is
//! resolved the way the name would be if it was written in the module which
//! holds that item: through the enclosing type, the module and the modules
//! which hold it, the prelude, and finally the crates which are known. So
//! `[u64]`, `[Vec::push]`, `[crate::option::Option]`, `[super::vec]` and
//! `[::std::iter]` all link to what they name.

use crate::alloc::prelude::*;
use crate::alloc::{self, String, Vec};
use crate::compile::Prelude;
use crate::doc::context::{Context, Kind, Meta};
use crate::item::ComponentRef;
use crate::{Item, ItemBuf};

use super::build::ItemKind;

/// The outcome of resolving a link.
#[derive(Debug)]
pub(crate) enum Link {
    /// The link text does not look like a path, such as `[-1, 1]` written in
    /// prose, so it was never meant to be a link and is left alone.
    NotAPath,
    /// The link looks like a path, but nothing by that name is in scope.
    Unresolved,
    /// The link resolved to the given target.
    Found(Target),
}

/// What a link resolved to.
#[derive(Debug)]
pub(crate) struct Target {
    /// The item which has a page of its own that the link points to.
    pub(crate) item: ItemBuf,
    /// The kind of page the item has.
    pub(crate) kind: ItemKind,
    /// The anchor on the page, like `method.push` or `variant.None`.
    pub(crate) fragment: Option<String>,
    /// What the link points to, for the title of the link.
    pub(crate) what: &'static str,
}

/// Restricts what a link may resolve to, through a suffix like `!` or `()`
/// or a prefix like `struct@`.
#[derive(Debug, Clone, Copy)]
enum Flavor {
    Any,
    Macro,
    Callable,
    Struct,
    Enum,
    Trait,
    Type,
    Module,
    Function,
    Const,
    Variant,
    Value,
}

impl Flavor {
    fn from_disambiguator(s: &str) -> Option<Self> {
        Some(match s {
            "struct" => Flavor::Struct,
            "enum" => Flavor::Enum,
            "trait" => Flavor::Trait,
            "type" | "prim" | "primitive" => Flavor::Type,
            "mod" | "module" => Flavor::Module,
            "macro" => Flavor::Macro,
            "fn" | "function" | "method" => Flavor::Function,
            "const" | "constant" => Flavor::Const,
            "variant" => Flavor::Variant,
            "value" => Flavor::Value,
            _ => return None,
        })
    }

    fn accepts(self, kind: &Kind<'_>) -> bool {
        match self {
            Flavor::Any => true,
            Flavor::Macro => matches!(kind, Kind::Macro),
            Flavor::Callable => matches!(kind, Kind::Function(..) | Kind::Variant | Kind::Struct),
            Flavor::Struct => matches!(kind, Kind::Struct),
            Flavor::Enum => matches!(kind, Kind::Enum),
            Flavor::Trait => matches!(kind, Kind::Trait),
            Flavor::Type => matches!(kind, Kind::Type | Kind::Struct | Kind::Enum),
            Flavor::Module => matches!(kind, Kind::Module),
            Flavor::Function => matches!(kind, Kind::Function(..)),
            Flavor::Const => matches!(kind, Kind::Const(..)),
            Flavor::Variant => matches!(kind, Kind::Variant),
            Flavor::Value => matches!(kind, Kind::Function(..) | Kind::Const(..) | Kind::Variant),
        }
    }
}

/// A link which has been taken apart.
struct Parsed<'a> {
    flavor: Flavor,
    absolute: bool,
    segments: Vec<&'a str>,
    fragment: Option<&'a str>,
}

/// Take a link apart, returning `None` if it does not look like a path.
fn parse(link: &str) -> alloc::Result<Option<Parsed<'_>>> {
    let link = link.trim().trim_matches('`');

    let (link, fragment) = match link.split_once('#') {
        Some((link, fragment)) => (link, Some(fragment)),
        None => (link, None),
    };

    if fragment.is_some_and(|f| f.is_empty() || f.contains(char::is_whitespace)) {
        return Ok(None);
    }

    let (mut flavor, link) = match link.split_once('@') {
        Some((prefix, link)) => match Flavor::from_disambiguator(prefix) {
            Some(flavor) => (flavor, link),
            None => return Ok(None),
        },
        None => (Flavor::Any, link),
    };

    let mut link = link;

    if let Some(rest) = link.strip_suffix('!') {
        link = rest;
        flavor = Flavor::Macro;
    } else if let Some(rest) = link.strip_suffix(')') {
        // Both `[Vec::new()]` and `[Some(v)]`, which names what is called
        // rather than the call.
        let Some((rest, _)) = rest.split_once('(') else {
            return Ok(None);
        };

        link = rest;

        if matches!(flavor, Flavor::Any) {
            flavor = Flavor::Callable;
        }
    }

    // Generic parameters, as in `[Result<T, E>]`, name the type they are on.
    if let Some(rest) = link.strip_suffix('>') {
        let Some((rest, _)) = rest.split_once('<') else {
            return Ok(None);
        };

        link = rest;
    }

    let (absolute, link) = match link.strip_prefix("::") {
        Some(link) => (true, link),
        None => (false, link),
    };

    let mut segments = Vec::new();

    for segment in link.split("::") {
        if !is_ident(segment) {
            return Ok(None);
        }

        segments.try_push(segment)?;
    }

    Ok(Some(Parsed {
        flavor,
        absolute,
        segments,
        fragment,
    }))
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();

    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_alphabetic() || first == '_') && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Resolve `link`, written in the documentation of `scope`.
pub(crate) fn resolve(
    context: &Context<'_>,
    prelude: &Prelude,
    scope: Meta<'_>,
    link: &str,
) -> alloc::Result<Link> {
    let Some(parsed) = parse(link)? else {
        return Ok(Link::NotAPath);
    };

    let Some((&first, rest)) = parsed.segments.split_first() else {
        return Ok(Link::NotAPath);
    };

    let target = if parsed.absolute {
        lookup(context, &ItemBuf::with_crate(first)?, rest, parsed.flavor)?
    } else {
        match first {
            "crate" => match scope.item.first() {
                Some(c) => lookup(context, &ItemBuf::with_item([c])?, rest, parsed.flavor)?,
                None => None,
            },
            "self" => {
                let module = module_of(context, scope)?;
                lookup(context, module, rest, parsed.flavor)?
            }
            "super" => {
                let mut module = module_of(context, scope)?.parent();
                let mut rest = rest;

                while let Some((&"super", tail)) = rest.split_first() {
                    module = module.and_then(Item::parent);
                    rest = tail;
                }

                match module {
                    Some(module) => lookup(context, module, rest, parsed.flavor)?,
                    None => None,
                }
            }
            "Self" => match self_type_of(context, scope)? {
                Some(ty) => lookup(context, ty, rest, parsed.flavor)?,
                None => None,
            },
            _ => lookup_in_scope(context, prelude, scope, &parsed.segments, parsed.flavor)?,
        }
    };

    let Some(mut target) = target else {
        return Ok(Link::Unresolved);
    };

    if let Some(fragment) = parsed.fragment {
        target.fragment = Some(fragment.try_to_owned()?);
    }

    Ok(Link::Found(target))
}

/// Look a relative path up the way a name written in `scope` would be.
fn lookup_in_scope(
    context: &Context<'_>,
    prelude: &Prelude,
    scope: Meta<'_>,
    segments: &[&str],
    flavor: Flavor,
) -> alloc::Result<Option<Target>> {
    // What an associated function or variant is documented under is its type,
    // so the names on that type are in scope as well, which is what lets the
    // documentation of `Peekable` say `[peek]`.
    if let Some(ty) = self_type_of(context, scope)? {
        if let Some(target) = lookup(context, ty, segments, flavor)? {
            return Ok(Some(target));
        }
    }

    let mut module = Some(module_of(context, scope)?);

    while let Some(m) = module {
        if let Some(target) = lookup(context, m, segments, flavor)? {
            return Ok(Some(target));
        }

        module = m.parent();
    }

    if let Some((&first, rest)) = segments.split_first() {
        if let Some(item) = prelude.get(first) {
            if let Some(target) = lookup(context, item, rest, flavor)? {
                return Ok(Some(target));
            }
        }

        if let Some(target) = lookup(context, &ItemBuf::with_crate(first)?, rest, flavor)? {
            return Ok(Some(target));
        }
    }

    Ok(None)
}

/// The module whose names are in scope in the documentation of `scope`.
fn module_of<'a>(context: &Context<'_>, scope: Meta<'a>) -> alloc::Result<&'a Item> {
    let mut item = Some(scope.item);

    while let Some(current) = item {
        if context
            .meta(current)?
            .iter()
            .any(|m| matches!(m.kind, Kind::Module))
        {
            return Ok(current);
        }

        item = current.parent();
    }

    // Nothing up the path is known to be a module, which happens for the
    // items of a source which was not visited as a whole. The parent is the
    // closest thing to one.
    Ok(scope.item.parent().unwrap_or(scope.item))
}

/// The type `Self` refers to in the documentation of `scope`, if any.
fn self_type_of<'a>(context: &Context<'_>, scope: Meta<'a>) -> alloc::Result<Option<&'a Item>> {
    if is_type(&scope.kind) {
        return Ok(Some(scope.item));
    }

    if let Some(parent) = scope.item.parent() {
        if matches!(
            scope.kind,
            Kind::Function(..) | Kind::Variant | Kind::Const(..)
        ) && page_kind(context, parent)?.is_some()
        {
            return Ok(Some(parent));
        }
    }

    Ok(None)
}

fn is_type(kind: &Kind<'_>) -> bool {
    matches!(kind, Kind::Type | Kind::Struct | Kind::Enum | Kind::Trait)
}

/// The kind of page a type has, if `item` is a type.
fn page_kind(context: &Context<'_>, item: &Item) -> alloc::Result<Option<ItemKind>> {
    for meta in context.meta(item)? {
        let kind = match meta.kind {
            Kind::Type => ItemKind::Type,
            Kind::Struct => ItemKind::Struct,
            Kind::Enum => ItemKind::Enum,
            Kind::Trait => ItemKind::Trait,
            _ => continue,
        };

        return Ok(Some(kind));
    }

    Ok(None)
}

/// Look up `base::segments`, and what page it is on.
fn lookup(
    context: &Context<'_>,
    base: &Item,
    segments: &[&str],
    flavor: Flavor,
) -> alloc::Result<Option<Target>> {
    let item = base.join(segments)?;

    let mut best = None::<(u8, Target)>;

    for meta in context.meta(&item)? {
        if !flavor.accepts(&meta.kind) {
            continue;
        }

        let Some((rank, target)) = target_of(context, &item, &meta.kind)? else {
            continue;
        };

        // A name can be more than one thing, like `u64` which is both a type
        // and a module of functions on it. The type is what is meant.
        if best.as_ref().is_none_or(|(best, _)| rank < *best) {
            best = Some((rank, target));
        }
    }

    Ok(best.map(|(_, target)| target))
}

/// The page `item` is documented on, ranked by how much it is what a name is
/// expected to mean when it is more than one thing.
fn target_of(
    context: &Context<'_>,
    item: &Item,
    kind: &Kind<'_>,
) -> alloc::Result<Option<(u8, Target)>> {
    let own = |kind: ItemKind, what: &'static str| -> alloc::Result<Target> {
        Ok(Target {
            item: item.try_to_owned()?,
            kind,
            fragment: None,
            what,
        })
    };

    let on_parent = |prefix: &str, what: &'static str| -> alloc::Result<Option<Target>> {
        let (Some(parent), Some(name)) = (item.parent(), item.last()) else {
            return Ok(None);
        };

        let Some(kind) = page_kind(context, parent)? else {
            return Ok(None);
        };

        let fragment = match name {
            ComponentRef::Str(name) if !prefix.is_empty() => Some(try_format!("{prefix}.{name}")),
            _ => None,
        };

        Ok(Some(Target {
            item: parent.try_to_owned()?,
            kind,
            fragment,
            what,
        }))
    };

    Ok(Some(match kind {
        Kind::Type => (0, own(ItemKind::Type, "type")?),
        Kind::Struct => (0, own(ItemKind::Struct, "struct")?),
        Kind::Enum => (0, own(ItemKind::Enum, "enum")?),
        Kind::Trait => (0, own(ItemKind::Trait, "trait")?),
        Kind::Variant => match on_parent("variant", "variant")? {
            Some(target) => (1, target),
            None => return Ok(None),
        },
        // An associated function is documented on the page of its type, and
        // has no page of its own.
        Kind::Function(..) => match on_parent("method", "method")? {
            Some(target) => (1, target),
            None => (1, own(ItemKind::Function, "function")?),
        },
        Kind::Macro => (2, own(ItemKind::Macro, "macro")?),
        // Constants have no page of their own, nor an entry on any other, so
        // the closest thing is the page of what holds them.
        Kind::Const(..) => match on_parent("", "constant")? {
            Some(target) => (2, target),
            None => match item.parent() {
                Some(parent) => (
                    2,
                    Target {
                        item: parent.try_to_owned()?,
                        kind: ItemKind::Module,
                        fragment: None,
                        what: "constant",
                    },
                ),
                None => return Ok(None),
            },
        },
        Kind::Module => (3, own(ItemKind::Module, "module")?),
        Kind::Unsupported => return Ok(None),
    }))
}
