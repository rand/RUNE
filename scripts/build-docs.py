#!/usr/bin/env python3
"""
Custom documentation build system for RUNE.
Parses markdown files and generates static HTML with custom design.
"""

import os
import shutil
from pathlib import Path
import markdown
from jinja2 import Environment, FileSystemLoader
import re

# Project configuration
PROJECT_NAME = "RUNE"
PROJECT_VERSION = "v0.1.0"
PROJECT_TAGLINE = "// High-Performance Authorization"
PROJECT_GLYPH = "∮"  # Closed contour integral
PROJECT_ACCENT_COLOR = "#7B1FA2"  # Purple
GITHUB_URL = "https://github.com/rand/RUNE"
SITE_URL = "https://rand.github.io/RUNE/"

# Directories
BASE_DIR = Path(__file__).parent.parent
SOURCE_DIR = BASE_DIR  # Markdown source files in root
TEMPLATES_DIR = BASE_DIR / "templates"
SITE_DIR = BASE_DIR / "docs"  # Output to docs/ for GitHub Pages

# Navigation structure
NAV_LINKS = [
    {"title": "Whitepaper", "href": "whitepaper.html"},
    {"title": "Agent Guide", "href": "agent-guide.html"},
    {"title": "GitHub", "href": GITHUB_URL, "external": True},
]


def setup_markdown():
    """Configure markdown parser with extensions."""
    return markdown.Markdown(
        extensions=[
            "extra",  # Tables, fenced code, etc.
            "codehilite",  # Syntax highlighting
            "toc",  # Table of contents
            "sane_lists",  # Better list handling
        ],
        extension_configs={
            "codehilite": {
                "css_class": "highlight",
                "linenums": False,
            },
            "toc": {
                "permalink": False,  # Disable permalink symbols
                "toc_depth": 3,
            },
        },
    )


def copy_static_files():
    """Ensure static assets are in place."""
    # For RUNE: static files are already in docs/ (SITE_DIR)
    # Just verify they exist
    static_dirs = ["css", "js", "assets"]

    all_present = all((SITE_DIR / dir_name).exists() for dir_name in static_dirs)

    if all_present:
        print("  Static assets already in place")
    else:
        print("  ⚠️  Some static assets missing - check docs/ folder")


def strip_yaml_frontmatter(content):
    """Remove YAML front matter from markdown content."""
    if content.startswith("---"):
        parts = content.split("---", 2)
        if len(parts) >= 3:
            return parts[2].strip()
    return content


def render_page(template_env, md_parser, template_name, md_file, output_file, extra_context=None):
    """Render a single page from markdown to HTML."""
    # Read markdown content
    md_path = SOURCE_DIR / md_file
    if not md_path.exists():
        print(f"  ⚠️  Skipping {md_file} (not found)")
        return

    with open(md_path, "r", encoding="utf-8") as f:
        md_content = f.read()

    # Strip YAML front matter if present
    md_content = strip_yaml_frontmatter(md_content)

    # Parse markdown to HTML
    html_content = md_parser.convert(md_content)
    toc = md_parser.toc if hasattr(md_parser, "toc") else ""

    # Reset markdown parser for next file
    md_parser.reset()

    # Prepare template context
    context = {
        "project_name": PROJECT_NAME,
        "project_version": PROJECT_VERSION,
        "project_tagline": PROJECT_TAGLINE,
        "project_glyph": PROJECT_GLYPH,
        "github_url": GITHUB_URL,
        "site_url": SITE_URL,
        "nav_links": NAV_LINKS,
        "content": html_content,
        "toc": toc,
    }

    if extra_context:
        context.update(extra_context)

    # Render template
    template = template_env.get_template(template_name)
    html_output = template.render(**context)

    # Write to site directory
    output_path = SITE_DIR / output_file
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(html_output)

    print(f"  ✓ {md_file} → {output_file}")


def build():
    """Main build function."""
    print(f"\n🔨 Building {PROJECT_NAME} documentation...\n")

    # Clean old HTML files (but keep static assets!)
    for html_file in SITE_DIR.glob("*.html"):
        html_file.unlink()

    # Ensure site directory exists
    SITE_DIR.mkdir(parents=True, exist_ok=True)

    # Setup Jinja2 environment
    template_env = Environment(loader=FileSystemLoader(str(TEMPLATES_DIR)))

    # Setup markdown parser
    md_parser = setup_markdown()

    # Render pages
    print("Rendering pages:")
    render_page(template_env, md_parser, "index.html", "README.md", "index.html")
    render_page(template_env, md_parser, "whitepaper.html", "WHITEPAPER.md", "whitepaper.html")
    render_page(template_env, md_parser, "whitepaper.html", "AGENT_GUIDE.md", "agent-guide.html")

    # Copy static files
    print("\nCopying static assets:")
    copy_static_files()

    # Create .nojekyll file to disable GitHub Pages Jekyll processing
    (SITE_DIR / ".nojekyll").touch()
    print("  ✓ Created .nojekyll")

    print(f"\n✅ Build complete! Site generated in: {SITE_DIR}\n")
    print(f"To preview locally:")
    print(f"  cd {SITE_DIR}")
    print(f"  python -m http.server 8000\n")


if __name__ == "__main__":
    build()
