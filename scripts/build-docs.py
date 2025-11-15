#!/usr/bin/env python3
"""
Build documentation for GitHub Pages deployment.

This script copies the pre-built documentation from the docs/ directory
to the site/ directory for GitHub Pages deployment.
"""

import os
import shutil
from pathlib import Path

def main():
    # Get the project root directory
    script_dir = Path(__file__).parent
    project_root = script_dir.parent

    # Define source and destination directories
    docs_dir = project_root / "docs"
    site_dir = project_root / "site"

    # Check if docs directory exists
    if not docs_dir.exists():
        print(f"Error: Documentation directory not found at {docs_dir}")
        return 1

    # Remove existing site directory if it exists
    if site_dir.exists():
        print(f"Removing existing site directory: {site_dir}")
        shutil.rmtree(site_dir)

    # Copy docs to site
    print(f"Copying documentation from {docs_dir} to {site_dir}")
    shutil.copytree(docs_dir, site_dir)

    print("Documentation build complete!")
    return 0

if __name__ == "__main__":
    exit(main())