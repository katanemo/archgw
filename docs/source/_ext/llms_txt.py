from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    # Only for type-checkers; Sphinx is only required in the docs build environment.
    from sphinx.application import Sphinx  # type: ignore[import-not-found]


@dataclass(frozen=True)
class LlmsTxtDoc:
    docname: str
    title: str
    text: str


def _iter_docs(app: Sphinx) -> Iterable[LlmsTxtDoc]:
    env = app.env

    # Sphinx internal pages that shouldn't be included.
    excluded = {"genindex", "search"}

    for docname in sorted(d for d in env.found_docs if d not in excluded):
        title_node = env.titles.get(docname)
        title = title_node.astext().strip() if title_node else docname

        doctree = env.get_doctree(docname)
        text = doctree.astext().strip()

        yield LlmsTxtDoc(docname=docname, title=title, text=text)


def _render_llms_txt(app: Sphinx) -> str:
    now = datetime.now(timezone.utc).isoformat()

    project = str(getattr(app.config, "project", "")).strip()
    release = str(getattr(app.config, "release", "")).strip()
    header = f"{project} {release}".strip() or "Documentation"

    docs = list(_iter_docs(app))

    lines: list[str] = []
    lines.append(header)
    lines.append("llms.txt (auto-generated)")
    lines.append(f"Generated (UTC): {now}")
    lines.append("")
    lines.append("Table of contents")
    for d in docs:
        lines.append(f"- {d.title} ({d.docname})")
    lines.append("")

    for d in docs:
        lines.append(d.title)
        lines.append("-" * max(3, len(d.title)))
        lines.append(f"Doc: {d.docname}")
        lines.append("")
        if d.text:
            lines.append(d.text)
        else:
            lines.append("(empty)")
        lines.append("")
        lines.append("---")
        lines.append("")

    return "\n".join(lines).replace("\r\n", "\n").strip() + "\n"


def _on_build_finished(app: Sphinx, exception: Exception | None) -> None:
    if exception is not None:
        return

    # Only generate for HTML-like builders where app.outdir is a website root.
    if getattr(app.builder, "format", None) != "html":
        return

    # Per repo convention, place generated artifacts under an `includes/` folder.
    out_path = Path(app.outdir) / "includes" / "llms.txt"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(_render_llms_txt(app), encoding="utf-8")


def setup(app: Sphinx) -> dict[str, object]:
    app.connect("build-finished", _on_build_finished)
    return {
        "version": "0.1.0",
        "parallel_read_safe": True,
        "parallel_write_safe": True,
    }
