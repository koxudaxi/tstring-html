from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]


def run_example(path: str) -> str:
    return subprocess.check_output(
        [sys.executable, str(REPO_ROOT / path)],
        cwd=REPO_ROOT,
        text=True,
    ).strip()


def test_html_example_runs() -> None:
    assert '<main class="page">' in run_example("examples/html_page.py")


def test_thtml_example_runs() -> None:
    assert run_example("examples/thtml_components.py") == (
        '<div><span class="badge badge-info">cached</span></div>'
    )


def test_html_account_settings_example_runs() -> None:
    output = run_example("examples/html_account_settings.py")
    assert 'class="settings-card theme-dark settings-card--pro"' in output
    assert "Maintainer of parser-first template tooling." in output


def test_html_search_results_example_runs() -> None:
    output = run_example("examples/html_search_results.py")
    assert '<main class="search-page">' in output
    assert "<mark>html</mark>" in output


def test_thtml_dashboard_example_runs() -> None:
    output = run_example("examples/thtml_dashboard.py")
    assert 'class="dashboard-shell"' in output
    assert 'class="badge badge-success"' in output
