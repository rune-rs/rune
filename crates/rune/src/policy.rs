use crate::alloc;
use crate::alloc::HashMap;
use crate::rc::Rc;

/// The policy to apply.
#[derive(Clone, Copy)]
pub(crate) enum Policy {
    /// Allow the given action.
    Allow,
    /// Warn about the given action.
    Warn,
    /// Deny the given action.
    Deny,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum Name {
    PatternMightPanic,
    Unused,
    Unreachable,
}

impl Name {
    /// Return the default policy for this name.
    #[inline]
    pub(crate) fn default(self) -> Policy {
        match self {
            Name::PatternMightPanic => Policy::Warn,
            Name::Unused => Policy::Warn,
            Name::Unreachable => Policy::Warn,
        }
    }
}

struct Inner {
    pub(crate) root: Option<Rc<Inner>>,
    pub(crate) map: HashMap<Name, Policy>,
}

pub(crate) struct Policies {
    inner: Rc<Inner>,
}

impl Policies {
    /// Lookup a policy by name.
    pub(crate) fn find(&self, name: Name) -> Policy {
        let mut this = &*self.inner;

        loop {
            if let Some(policy) = this.map.get(&name) {
                return *policy;
            }

            let Some(parent) = this.root.as_ref() else {
                return name.default();
            };

            this = parent;
        }
    }
}

impl Policies {
    /// Construct a new set of default policies.
    #[inline]
    pub(crate) fn new() -> alloc::Result<Self> {
        Ok(Self {
            inner: Rc::try_new(Inner {
                root: None,
                map: HashMap::new(),
            })?,
        })
    }
}
