# RUNE GitHub Pages Documentation

This directory contains the source for the RUNE GitHub Pages site, which includes the technical whitepaper and documentation.

## Local Development

### Prerequisites

- Ruby 2.7+
- Bundler

### Setup

```bash
cd docs

# Install dependencies
bundle install

# Serve locally
bundle exec jekyll serve

# Open http://localhost:4000 in your browser
```

### Live Reload

Jekyll watches for file changes and rebuilds automatically. Simply refresh your browser to see updates.

## Structure

```
docs/
├── _config.yml          # Jekyll configuration
├── Gemfile              # Ruby dependencies
├── index.md             # Home page
├── whitepaper.md        # Whitepaper page (includes ../WHITEPAPER.md)
├── agent-guide.md       # Agent guide page (includes ../AGENT_GUIDE.md)
├── assets/
│   └── css/
│       └── style.scss   # Custom RUNE styling
└── README.md            # This file
```

## Deployment

GitHub Pages automatically builds and deploys from the `docs/` directory when configured in repository settings.

### Enable GitHub Pages

1. Go to repository Settings
2. Navigate to Pages
3. Source: Deploy from a branch
4. Branch: `main` (or your default branch)
5. Folder: `/docs`
6. Save

The site will be available at: `https://yourusername.github.io/rune`

## Customization

### Colors

The RUNE color palette is defined in `assets/css/style.scss`:

- Primary (Purple): `#7B1FA2`
- Secondary (Blue): `#0277BD`
- Accent (Amber): `#F57F17`
- Success (Green): `#388E3C`
- Danger (Pink): `#C2185B`

### Content

- **Home page**: Edit `index.md`
- **Whitepaper**: Edit `../WHITEPAPER.md` (automatically included)
- **Agent guide**: Edit `../AGENT_GUIDE.md` (automatically included)

### Navigation

Edit the `nav_links` section in `_config.yml` to add or remove navigation items.

## Dependencies

- **Jekyll 4.3**: Static site generator
- **Minima 2.5**: Base theme
- **jekyll-feed**: RSS feed generation
- **jekyll-seo-tag**: SEO meta tags
- **jekyll-sitemap**: XML sitemap generation

## Troubleshooting

### Bundler errors

```bash
# Update bundler
gem install bundler

# Clean and reinstall
rm -rf Gemfile.lock
bundle install
```

### Port already in use

```bash
# Kill existing Jekyll server
pkill -f jekyll

# Or use a different port
bundle exec jekyll serve --port 4001
```

## Resources

- [Jekyll Documentation](https://jekyllrb.com/docs/)
- [Minima Theme](https://github.com/jekyll/minima)
- [GitHub Pages Documentation](https://docs.github.com/en/pages)
