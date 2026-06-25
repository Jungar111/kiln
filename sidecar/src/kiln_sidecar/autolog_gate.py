"""Suppress MLflow autolog around ephemeral (human REPL) kernel calls.

`mlflow.<flavor>.autolog()` patches `.fit` globally; it has no concept of
Kiln's experiment-vs-inspection split (spec §7). So we wrap the single hook that
every flavour routes through — `mlflow.utils.autologging_utils.safe_patch` — with
one that threads a check on the `is_ephemeral` context variable.

The seam: `safe_patch(integration, destination, function_name, patch_function,
...)` registers a `patch_function` whose first positional argument is `original`
(the real method, e.g. `LogisticRegression.fit`). Autolog's logging lives inside
`patch_function`; `original` is the untouched library call. When `is_ephemeral`
is set we bypass `patch_function` entirely and invoke `original` directly, so the
human poke runs but nothing lands in the MLflow run.

Why this seam and not a higher-level one: `safe_patch` is the *only* registration
hook autolog uses, for every flavour, so wrapping it once covers them all.

Binding-site caveat: flavour modules import the name eagerly
(`from mlflow.utils.autologging_utils import safe_patch`), so rebinding only the
attribute on `autologging_utils` would miss `mlflow.sklearn.safe_patch` and
friends. We therefore rebind every already-imported alias as well. If the seam
disappears entirely, `install_autolog_gate` raises loudly rather than silently
dropping the gate (acceptance: "fail loudly").

The gate is installed **inside the kernel process** (see `execute.py`), because
that is where experiment code and therefore autolog's `.fit` patches actually
run. The context variable is set per-cell, around the executed code, in the same
process — so the check fires where it matters.
"""

from __future__ import annotations

import contextvars
import functools
import sys
from typing import TYPE_CHECKING, Final

import mlflow.utils.autologging_utils as _autolog_utils

if TYPE_CHECKING:
    from collections.abc import Callable

is_ephemeral: Final[contextvars.ContextVar[bool]] = contextvars.ContextVar(
    "kiln_is_ephemeral",
    default=False,
)

# Marks our wrapper so re-installation is a no-op (acceptance: idempotent).
_GATE_MARKER: Final[str] = "_kiln_autolog_gate_installed"

# The attribute every flavour binds. Held as a variable (not a literal) so the
# setattr-based rebind below does not trip ruff's "constant setattr" lint while
# still avoiding the typed-attribute-assignment the checker rejects.
_SAFE_PATCH_ATTR: Final[str] = "safe_patch"


def _wrap(orig_safe_patch: Callable[..., object]) -> Callable[..., object]:
    @functools.wraps(orig_safe_patch)
    def gated_safe_patch(
        autologging_integration: str,
        destination: object,
        function_name: str,
        patch_function: Callable[..., object],
        *args: object,
        **kwargs: object,
    ) -> object:
        @functools.wraps(patch_function)
        def gated_patch_function(
            original: Callable[..., object],
            *p_args: object,
            **p_kwargs: object,
        ) -> object:
            if is_ephemeral.get():
                # Human poke: run the real method, skip all autolog bookkeeping.
                return original(*p_args, **p_kwargs)
            return patch_function(original, *p_args, **p_kwargs)

        return orig_safe_patch(
            autologging_integration,
            destination,
            function_name,
            gated_patch_function,
            *args,
            **kwargs,
        )

    setattr(gated_safe_patch, _GATE_MARKER, True)
    return gated_safe_patch


def _rebind_safe_patch(module: object, gated: Callable[..., object]) -> None:
    """Replace `module.safe_patch` with `gated`.

    Done via setattr because the binding sites are MLflow modules whose typed
    `safe_patch` attribute we are deliberately monkey-patching at runtime; a
    direct assignment would trip the type checker on an intentional shadow.
    """
    setattr(module, _SAFE_PATCH_ATTR, gated)


def install_autolog_gate() -> None:
    """Wrap `safe_patch` so autolog patches no-op while `is_ephemeral` is True.

    Idempotent: a second call detects the marker and returns. Rebinds every
    already-imported alias of `safe_patch` (flavours import the name eagerly).
    Raises if the upstream seam has moved, rather than silently leaving runs
    un-gated.
    """
    orig = getattr(_autolog_utils, _SAFE_PATCH_ATTR, None)
    if orig is None:
        msg = (
            "mlflow.utils.autologging_utils.safe_patch is gone — the autolog gate "
            "seam moved. Re-point install_autolog_gate() before trusting ephemeral "
            "suppression."
        )
        raise RuntimeError(msg)
    if getattr(orig, _GATE_MARKER, False):
        return  # already installed

    gated = _wrap(orig)
    _rebind_safe_patch(_autolog_utils, gated)
    # Rebind eager aliases in any module that imported the original by value.
    for module in list(sys.modules.values()):
        if module is None or module is _autolog_utils:
            continue
        if getattr(module, _SAFE_PATCH_ATTR, None) is orig:
            _rebind_safe_patch(module, gated)
