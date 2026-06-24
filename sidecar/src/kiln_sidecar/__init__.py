"""Kiln Python sidecar.

Hosts the IPython kernel, an Arrow IPC server for DataFrame paging,
and the thin MLflow run-lifecycle layer. Spawned and supervised by
the Rust core (Tauri).
"""

__version__ = "0.1.0"
