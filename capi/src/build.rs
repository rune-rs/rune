use std::ptr;
use std::sync::Arc;

use crate::{
    Context, Diagnostics, InternalContext, InternalDiagnostics, InternalUnit, Sources, Unit,
};

/// Prepare a build.
#[repr(C)]
pub struct Build {
    sources: *mut Sources,
    context: Option<ptr::NonNull<Context>>,
    diagnostics: Option<ptr::NonNull<Diagnostics>>,
}

/// Prepare a new build.
#[no_mangle]
pub extern "C" fn rune_build_prepare(sources: *mut Sources) -> Build {
    Build {
        sources,
        context: None,
        diagnostics: None,
    }
}

/// Associate a context with the build.
///
/// # Safety
///
/// Must be called with a `build` argument that has been setup with
/// [rune_build_prepare] and a `context` that has been allocated with
/// [rune_context_new][crate::rune_context_new].
#[no_mangle]
pub unsafe extern "C" fn rune_build_with_context(mut build: *mut Build, context: *mut Context) {
    (*build).context = ptr::NonNull::new(context);
}

/// Associate diagnostics with the build.
///
/// # Safety
///
/// Must be called with a `build` argument that has been setup with
/// [rune_build_prepare] and a `diagnostics` that has been allocated with
/// [rune_diagnostics_new][crate::rune_diagnostics_new].
#[no_mangle]
pub unsafe extern "C" fn rune_build_with_diagnostics(
    mut build: *mut Build,
    diagnostics: *mut Diagnostics,
) {
    (*build).diagnostics = ptr::NonNull::new(diagnostics);
}

/// Perform a build.
///
/// On a successful returns `true` and sets `unit` to the newly allocated unit.
/// Any old unit present will be de-allocated.
/// Otherwise the `unit` argument is left alone.
///
/// # Safety
///
/// Must be called with a `build` argument that has been setup with
/// [rune_build_prepare] and a `unit` that has been allocated with
/// [rune_unit_new][crate::rune_unit_new].
#[no_mangle]
pub unsafe extern "C" fn rune_build_build(build: *mut Build, unit: *mut Unit) -> bool {
    let build = &mut *build;
    let sources = &mut *(build.sources as *mut rune::Sources);
    let b = rune::prepare(sources);

    let b = if let Some(context) = build
        .context
        .as_mut()
        .and_then(|c| (*c.as_ptr().cast::<InternalContext>()).as_mut())
    {
        b.with_context(context)
    } else {
        b
    };

    let b = if let Some(diagnostics) = build
        .diagnostics
        .as_mut()
        .and_then(|d| (*d.as_ptr().cast::<InternalDiagnostics>()).as_mut())
    {
        b.with_diagnostics(diagnostics)
    } else {
        b
    };

    if let Ok(out) = b.build() {
        let _ = ptr::replace(unit as *mut InternalUnit, Some(Arc::new(out)));
        true
    } else {
        false
    }
}
